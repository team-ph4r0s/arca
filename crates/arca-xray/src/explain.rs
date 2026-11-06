// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

//! Static behaviour and consequence analysis: the text behind the "More" button.
//!
//! For a macro, [`explain_macro`] reads the source **without running it**: it folds string
//! concatenation (`"ht" & "tp"`) and `Chr(..)` calls so that the real text shows, looks for the
//! calls that start programs, download, write files or redirect, collects the addresses, and
//! says in plain words (Russian and English) what the macro does and what it can lead to.
//! A macro that only counts the document's own tables, rows or pages is called harmless.
//!
//! For every other finding, [`explain_kind`] says what the object is and what it can lead to.

use crate::report::{Category, MacroIntent, MacroModule, ThreatKind};

/// A sentence in both languages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    /// Russian.
    pub ru: String,
    /// English.
    pub en: String,
}

impl Text {
    fn new(ru: impl Into<String>, en: impl Into<String>) -> Self {
        Self { ru: ru.into(), en: en.into() }
    }
}

/// How dangerous a macro looks after reading its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Works on the document's own content only.
    Harmless,
    /// Reads its surroundings, starts by itself or hides its text, but nothing proves harm.
    Suspicious,
    /// Starts programs, downloads, writes files or redirects.
    Hostile,
}

/// What an extracted indicator is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IocKind {
    /// A web address.
    Url,
    /// A network path (`\\host\share`): opening it leaks the user's credentials hash.
    Unc,
    /// An API, object or command the macro calls.
    Call,
    /// Anything else the analysis already collected (addresses, program names).
    Other,
}

/// An address, path or call found in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ioc {
    /// Kind.
    pub kind: IocKind,
    /// The value, display-safe and shortened.
    pub value: String,
}

/// The four parts of the "More" screen, except the source text itself (the macro carries it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    /// Verdict.
    pub verdict: Verdict,
    /// What the object is, in a few words.
    pub title: Text,
    /// What it can lead to.
    pub summary: Text,
    /// The static trace, step by step (nothing was executed).
    pub steps: Vec<Text>,
    /// Addresses, paths and calls.
    pub iocs: Vec<Ioc>,
}

const MAX_STEPS: usize = 24;
const MAX_IOCS: usize = 40;
const LINE_CHARS: usize = 120;

/// Reads the source of `module` and explains it.
#[must_use]
pub fn explain_macro(module: &MacroModule) -> Explanation {
    let (clean, folds) = deobfuscate(&module.source);
    let lower = clean.to_lowercase();
    let hits: Vec<&Call> = CALLS.iter().filter(|c| lower.contains(c.needle)).collect();
    let urls = find_urls(&clean);
    let uncs = find_uncs(&clean);

    let hostile_hit = hits.iter().any(|c| c.hostile);
    let redirect = !urls.is_empty() && hits.iter().any(|c| c.redirect);
    // Sending the reader to an address of the macro's choosing is hostile in itself.
    let verdict = if hostile_hit || redirect || module.intent == MacroIntent::Malicious {
        Verdict::Hostile
    } else if module.intent == MacroIntent::Benign
        && hits.is_empty()
        && urls.is_empty()
        && uncs.is_empty()
    {
        Verdict::Harmless
    } else {
        Verdict::Suspicious
    };

    let mut steps = Vec::new();
    if !module.auto_exec.is_empty() {
        let names = module.auto_exec.join(", ");
        steps.push(Text::new(
            format!("Запускается сам, без вашего нажатия: {names}."),
            format!("Starts by itself, without a click: {names}."),
        ));
    }
    if folds > 0 {
        steps.push(Text::new(
            format!(
                "Текст собран из кусков (склеек строк и вызовов Chr: {folds}). После сборки виден настоящий код."
            ),
            format!(
                "The text is assembled from pieces ({folds} joins and Chr calls). Put together, the real code shows."
            ),
        ));
    }
    for call in hits.iter().take(8) {
        let line = first_line_with(&clean, call.needle);
        steps.push(Text::new(
            format!("{}{}", call.ru, quoted(&line)),
            format!("{}{}", call.en, quoted(&line)),
        ));
    }
    for url in urls.iter().take(6) {
        steps.push(Text::new(
            format!("Обращается к адресу: {url}"),
            format!("Reaches the address: {url}"),
        ));
    }
    for unc in uncs.iter().take(3) {
        steps.push(Text::new(
            format!(
                "Открывает сетевой путь {unc}: система при этом отдаёт хэш пароля пользователя."
            ),
            format!("Opens the network path {unc}: Windows hands over the user's password hash doing so."),
        ));
    }
    if verdict == Verdict::Harmless {
        let counted = counted_things(&lower);
        steps.push(Text::new(
            format!("Только считает: {counted}. К программам, файлам и сети не обращается."),
            format!("It only counts: {counted}. It never touches programs, files or the network."),
        ));
    }
    steps.push(conclusion(verdict));
    steps.truncate(MAX_STEPS);

    let mut iocs: Vec<Ioc> = Vec::new();
    for url in &urls {
        push_ioc(&mut iocs, IocKind::Url, url);
    }
    for unc in &uncs {
        push_ioc(&mut iocs, IocKind::Unc, unc);
    }
    for call in &hits {
        push_ioc(&mut iocs, IocKind::Call, call.shown);
    }
    for other in &module.iocs {
        push_ioc(&mut iocs, IocKind::Other, other);
    }
    iocs.truncate(MAX_IOCS);

    let first_url = urls.first().cloned().unwrap_or_default();
    let (title, summary) = match verdict {
        Verdict::Harmless => (
            Text::new("Безобидный вычислительный макрос", "Harmless calculation macro"),
            Text::new(
                "Безобидный вычислительный макрос: считает элементы документа, не обращается к ОС или сети.",
                "Harmless calculation macro: counts the document's own elements and never touches the OS or the network.",
            ),
        ),
        Verdict::Suspicious => (
            Text::new("Подозрительный макрос", "Suspicious macro"),
            Text::new(
                "Вреда не доказано, но макрос запускается сам, читает окружение или прячет свой текст. Не включайте макросы, если не ждёте их от отправителя.",
                "No harm is proven, but the macro starts by itself, reads its surroundings or hides its text. Do not enable macros you did not expect from the sender.",
            ),
        ),
        Verdict::Hostile if redirect => (
            Text::new("Макрос перенаправления на сайт", "Redirecting macro"),
            Text::new(
                format!(
                    "Способен скрытно перенаправить вас на вредоносный сайт {first_url}, скачать нагрузку или выполнить команду в системе."
                ),
                format!(
                    "Can silently redirect you to a malicious site {first_url}, download a payload or run a command on your system."
                ),
            ),
        ),
        Verdict::Hostile => (
            Text::new("Боевой макрос", "Hostile macro"),
            Text::new(
                "Способен запускать программы, скачивать и записывать файлы или менять систему от вашего имени.",
                "Can start programs, download and write files or change the system in your name.",
            ),
        ),
    };
    Explanation { verdict, title, summary, steps, iocs }
}

/// Says what a finding of this kind is and what it can lead to (for findings without source).
#[must_use]
pub fn explain_kind(kind: ThreatKind, detail: &str) -> Explanation {
    let (title, summary): (Text, Text) = match kind {
        ThreatKind::DdeField | ThreatKind::DdeFormula => (
            Text::new("Внедрение команды через DDE", "Command injection through DDE"),
            Text::new(
                "Поле DDE способно запустить программу (например cmd.exe или PowerShell) сразу после согласия на «обновить связи».",
                "A DDE field can start a program (such as cmd.exe or PowerShell) as soon as you agree to \"update links\".",
            ),
        ),
        ThreatKind::MacroAutoExec | ThreatKind::XlmAutoOpenName => (
            Text::new("Автозапуск макроса", "Macro auto-start"),
            Text::new(
                "Код запускается сам при открытии (AutoOpen, Document_Open, Auto_Open), как только вы разрешите макросы.",
                "The code runs by itself on opening (AutoOpen, Document_Open, Auto_Open) as soon as you allow macros.",
            ),
        ),
        ThreatKind::VbaMacros | ThreatKind::XlmMacroSheet | ThreatKind::StarBasicMacros => (
            Text::new("Макросы в документе", "Macros in the document"),
            Text::new(
                "Документ содержит программы. Они могут делать всё, что может пользователь: читать файлы, выходить в сеть, запускать программы.",
                "The document contains programs. They can do what the user can: read files, reach the network, start programs.",
            ),
        ),
        ThreatKind::RemoteTemplate => (
            Text::new(
                "Скрытый удалённый шаблон (утечка NTLM)",
                "Hidden remote template (NTLM leak)",
            ),
            Text::new(
                "При открытии документ загружает шаблон с чужого сервера: владелец сервера получает ваш IP и хэш пароля Windows и может подменить содержимое.",
                "On opening, the document loads a template from someone else's server: its owner learns your IP and Windows password hash and can swap the content.",
            ),
        ),
        ThreatKind::UncPathReference => (
            Text::new("Сетевой путь UNC", "UNC network path"),
            Text::new(
                "Обращение к \\\\сервер\\ресурс отдаёт серверу хэш пароля Windows без вашего ведома.",
                "Reaching \\\\server\\share hands the server your Windows password hash without your knowledge.",
            ),
        ),
        ThreatKind::ExternalImage => (
            Text::new("Внешняя картинка-«маячок»", "External tracking picture"),
            Text::new(
                "Картинка берётся с чужого сервера: по запросу видно, что вы открыли документ, когда и с какого адреса.",
                "The picture is fetched from someone else's server: the request shows that you opened the document, when and from which address.",
            ),
        ),
        ThreatKind::PdfJavaScript | ThreatKind::PdfLaunchAction | ThreatKind::PdfAutoAction => (
            Text::new("Действие PDF при открытии", "PDF action on opening"),
            Text::new(
                "PDF несёт скрипт или команду запуска, которые просмотрщик может выполнить при открытии.",
                "The PDF carries a script or a launch command that a viewer may run on opening.",
            ),
        ),
        ThreatKind::OlePackage
        | ThreatKind::EmbeddedExecutable
        | ThreatKind::ExploitableOleClass => (
            Text::new(
                "Вложенная программа или уязвимый объект",
                "Embedded program or exploitable object",
            ),
            Text::new(
                "Внутри лежит исполняемый файл или объект с известной уязвимостью; по двойному щелчку он может запуститься.",
                "An executable or an object with a known weakness is inside; a double click can start it.",
            ),
        ),
        _ => by_category(kind.category()),
    };
    let mut steps =
        vec![Text::new(format!("Найдено: {}.", kind.id()), format!("Found: {}.", kind.id()))];
    if !detail.is_empty() {
        let shown: String = detail.chars().take(LINE_CHARS).collect();
        steps.push(Text::new(format!("Деталь: {shown}"), format!("Detail: {shown}")));
    }
    steps.push(Text::new(
        "Ничего не запускалось и не загружалось: разбор статический.",
        "Nothing was run or fetched: the analysis is static.",
    ));
    let verdict = match kind.category() {
        Category::ActiveContent | Category::RemoteContent => Verdict::Hostile,
        Category::Concealment | Category::Structural | Category::Encryption => Verdict::Suspicious,
        Category::PrivacyLeak => Verdict::Harmless,
    };
    Explanation { verdict, title, summary, steps, iocs: Vec::new() }
}

fn by_category(category: Category) -> (Text, Text) {
    match category {
        Category::ActiveContent => (
            Text::new("Активное содержимое", "Active content"),
            Text::new(
                "Часть файла способна выполнять действия, а не только показывать данные.",
                "Part of the file can act, not just show data.",
            ),
        ),
        Category::RemoteContent => (
            Text::new("Обращение наружу", "Reaching outside"),
            Text::new(
                "Файл подгружает данные с внешнего адреса: так видно, что вы его открыли.",
                "The file loads data from an outside address, which shows that you opened it.",
            ),
        ),
        Category::Concealment => (
            Text::new("Скрытое содержимое", "Hidden content"),
            Text::new(
                "Часть файла спрятана от читателя, но лежит внутри.",
                "Part of the file is hidden from the reader but is inside.",
            ),
        ),
        Category::PrivacyLeak => (
            Text::new("Утечка личных данных", "Personal data leak"),
            Text::new(
                "В файле остались сведения об авторе, устройстве или месте, которые вы могли не хотеть раскрывать.",
                "The file keeps details about the author, device or place that you may not want to share.",
            ),
        ),
        Category::Structural => (
            Text::new("Нарушение структуры файла", "Irregular file structure"),
            Text::new(
                "Устройство файла отличается от нормы; так прячут вредный код или ломают программы чтения.",
                "The file's structure differs from the norm; this hides harmful code or breaks readers.",
            ),
        ),
        Category::Encryption => (
            Text::new("Зашифрованная часть", "Encrypted part"),
            Text::new(
                "Эту часть нельзя проверить без пароля, поэтому вердикт неполный.",
                "This part cannot be inspected without the password, so the verdict is incomplete.",
            ),
        ),
    }
}

// ---------------------------------------------------------------------------------------------
// The calls that matter
// ---------------------------------------------------------------------------------------------

struct Call {
    /// Lower-case text looked for in the unfolded source.
    needle: &'static str,
    /// How it is listed among the indicators.
    shown: &'static str,
    /// The step, up to the quoted line.
    ru: &'static str,
    en: &'static str,
    /// Starts programs, downloads, writes files or changes the system.
    hostile: bool,
    /// Can send the user to an address.
    redirect: bool,
}

const fn call(
    needle: &'static str,
    shown: &'static str,
    ru: &'static str,
    en: &'static str,
    hostile: bool,
    redirect: bool,
) -> Call {
    Call { needle, shown, ru, en, hostile, redirect }
}

const CALLS: &[Call] = &[
    call(
        "wscript.shell",
        "WScript.Shell",
        "Создаёт оболочку Windows (WScript.Shell), через которую запускаются команды: ",
        "Creates the Windows shell (WScript.Shell) that runs commands: ",
        true,
        true,
    ),
    call(
        "shell.application",
        "Shell.Application",
        "Использует Shell.Application для запуска файлов и адресов: ",
        "Uses Shell.Application to open files and addresses: ",
        true,
        true,
    ),
    call(
        "shell(",
        "Shell()",
        "Запускает программу или команду: ",
        "Starts a program or command: ",
        true,
        true,
    ),
    call(
        "powershell",
        "powershell",
        "Вызывает PowerShell, мощную оболочку, любимую вредоносным кодом: ",
        "Calls PowerShell, the powerful shell malware favours: ",
        true,
        false,
    ),
    call(
        "cmd.exe",
        "cmd.exe",
        "Вызывает командную строку cmd.exe: ",
        "Calls the cmd.exe command line: ",
        true,
        true,
    ),
    call(
        "cmd /c",
        "cmd /c",
        "Выполняет одну команду через cmd: ",
        "Runs one command through cmd: ",
        true,
        true,
    ),
    call(
        "mshta",
        "mshta",
        "Запускает mshta, исполнитель HTML-приложений: ",
        "Starts mshta, the HTML-application host: ",
        true,
        false,
    ),
    call(
        "certutil",
        "certutil",
        "Использует certutil, которым обычно скачивают файлы: ",
        "Uses certutil, which is often abused to download files: ",
        true,
        false,
    ),
    call(
        "bitsadmin",
        "bitsadmin",
        "Использует bitsadmin для фоновой загрузки: ",
        "Uses bitsadmin for background downloads: ",
        true,
        false,
    ),
    call(
        "regsvr32",
        "regsvr32",
        "Запускает код через regsvr32: ",
        "Runs code through regsvr32: ",
        true,
        false,
    ),
    call(
        "rundll32",
        "rundll32",
        "Запускает функцию библиотеки через rundll32: ",
        "Runs a library function through rundll32: ",
        true,
        false,
    ),
    call(
        "urldownloadtofile",
        "URLDownloadToFile",
        "Скачивает файл из сети на диск: ",
        "Downloads a file from the network to disk: ",
        true,
        false,
    ),
    call(
        "xmlhttp",
        "XMLHTTP",
        "Отправляет сетевой запрос (XMLHTTP): ",
        "Sends a network request (XMLHTTP): ",
        true,
        false,
    ),
    call(
        "winhttp",
        "WinHttp",
        "Отправляет сетевой запрос (WinHttp): ",
        "Sends a network request (WinHttp): ",
        true,
        false,
    ),
    call(
        "adodb.stream",
        "ADODB.Stream",
        "Записывает полученные данные в файл (ADODB.Stream): ",
        "Writes received data to a file (ADODB.Stream): ",
        true,
        false,
    ),
    call(
        "followhyperlink",
        "FollowHyperlink",
        "Открывает адрес в браузере (FollowHyperlink): ",
        "Opens an address in the browser (FollowHyperlink): ",
        false,
        true,
    ),
    call(
        "openredirecturl",
        "openRedirectURL",
        "Процедура перенаправления openRedirectURL: ",
        "The redirect procedure openRedirectURL: ",
        false,
        true,
    ),
    call(
        "regwrite",
        "RegWrite",
        "Пишет в реестр Windows: ",
        "Writes to the Windows registry: ",
        true,
        false,
    ),
    call(
        "environ(",
        "Environ()",
        "Читает переменные окружения: ",
        "Reads environment variables: ",
        false,
        false,
    ),
    call(
        "createobject",
        "CreateObject",
        "Создаёт внешний COM-объект: ",
        "Creates an external COM object: ",
        false,
        false,
    ),
];

/// What a harmless macro is seen counting.
fn counted_things(lower: &str) -> String {
    let names = [
        ("tables.count", "таблицы"),
        ("rows.count", "строки"),
        ("columns.count", "столбцы"),
        ("pages", "страницы"),
        ("paragraphs.count", "абзацы"),
        ("words.count", "слова"),
        ("sheets.count", "листы"),
    ];
    let found: Vec<&str> =
        names.iter().filter(|(k, _)| lower.contains(k)).map(|(_, v)| *v).collect();
    if found.is_empty() {
        "элементы самого документа".to_owned()
    } else {
        found.join(", ")
    }
}

fn conclusion(verdict: Verdict) -> Text {
    match verdict {
        Verdict::Harmless => Text::new(
            "Вывод: безобидно. Макрос не выходит за пределы документа.",
            "Conclusion: harmless. The macro stays inside the document.",
        ),
        Verdict::Suspicious => Text::new(
            "Вывод: подозрительно. Прямого вреда не видно, но доверять такому макросу нельзя.",
            "Conclusion: suspicious. No direct harm is visible, but such a macro should not be trusted.",
        ),
        Verdict::Hostile => Text::new(
            "Вывод: опасно. Макрос способен выполнить действия вне документа. Arca его не запускает.",
            "Conclusion: dangerous. The macro can act outside the document. Arca never runs it.",
        ),
    }
}

fn quoted(line: &str) -> String {
    if line.is_empty() { String::new() } else { format!("«{line}»") }
}

fn push_ioc(list: &mut Vec<Ioc>, kind: IocKind, value: &str) {
    let shown: String = value.chars().take(LINE_CHARS).collect();
    if shown.is_empty() || list.iter().any(|i| i.value == shown) {
        return;
    }
    list.push(Ioc { kind, value: shown });
}

fn first_line_with(source: &str, needle_lower: &str) -> String {
    source
        .lines()
        .find(|l| l.to_lowercase().contains(needle_lower))
        .map(|l| l.trim().chars().take(LINE_CHARS).collect())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------------------------
// Unfolding string tricks (text only; nothing is evaluated)
// ---------------------------------------------------------------------------------------------

/// Folds `Chr(104)` calls and `"a" & "b"` joins; returns the text and how many folds happened.
fn deobfuscate(source: &str) -> (String, usize) {
    let (step1, chr_count) = fold_chr(source);
    let (step2, join_count) = fold_joins(&step1);
    (step2, chr_count + join_count)
}

fn matches_ci(chars: &[char], at: usize, word: &str) -> bool {
    word.chars()
        .enumerate()
        .all(|(k, w)| chars.get(at + k).is_some_and(|c| c.to_ascii_lowercase() == w))
}

fn preceded_by_word(chars: &[char], at: usize) -> bool {
    at > 0 && chars.get(at - 1).is_some_and(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
}

/// Tries to read `Chr(n)` / `ChrW(n)` / `Chr$(n)` at `at`; returns the character and the index
/// after the closing parenthesis.
fn chr_call_at(chars: &[char], at: usize) -> Option<(char, usize)> {
    if !matches_ci(chars, at, "chr") || preceded_by_word(chars, at) {
        return None;
    }
    let mut j = at + 3;
    if chars.get(j).is_some_and(|c| c.eq_ignore_ascii_case(&'w')) {
        j += 1;
    }
    if chars.get(j) == Some(&'$') {
        j += 1;
    }
    if chars.get(j) != Some(&'(') {
        return None;
    }
    let mut k = j + 1;
    while chars.get(k) == Some(&' ') {
        k += 1;
    }
    let start = k;
    let mut code: u32 = 0;
    while let Some(d) = chars.get(k).and_then(|c| c.to_digit(10)) {
        code = code.saturating_mul(10).saturating_add(d);
        k += 1;
    }
    while chars.get(k) == Some(&' ') {
        k += 1;
    }
    if k == start || chars.get(k) != Some(&')') {
        return None;
    }
    // Only plain printable characters are folded; a quote or a control code stays a call.
    let ch = char::from_u32(code).filter(|c| (' '..='~').contains(c) && *c != '"')?;
    Some((ch, k + 1))
}

fn fold_chr(source: &str) -> (String, usize) {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut folds = 0;
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        if let Some((ch, next)) = chr_call_at(&chars, i) {
            out.push('"');
            out.push(ch);
            out.push('"');
            folds += 1;
            i = next;
        } else {
            out.push(c);
            i += 1;
        }
    }
    (out, folds)
}

fn fold_joins(source: &str) -> (String, usize) {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut folds = 0;
    let mut in_string = false;
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        if c == '"' {
            if in_string && chars.get(i + 1) == Some(&'"') {
                out.push_str("\"\"");
                i += 2;
                continue;
            }
            if in_string {
                // A closing quote: is it followed by `& "` or `+ "`?
                let mut j = i + 1;
                while matches!(chars.get(j), Some(' ' | '\t')) {
                    j += 1;
                }
                if matches!(chars.get(j), Some('&' | '+')) {
                    let mut k = j + 1;
                    while matches!(chars.get(k), Some(' ' | '\t')) {
                        k += 1;
                    }
                    if chars.get(k) == Some(&'"') {
                        folds += 1;
                        i = k + 1;
                        continue;
                    }
                }
            }
            in_string = !in_string;
        }
        out.push(c);
        i += 1;
    }
    (out, folds)
}

// ---------------------------------------------------------------------------------------------
// Addresses
// ---------------------------------------------------------------------------------------------

fn find_urls(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut found: Vec<String> = Vec::new();
    for scheme in ["http://", "https://", "ftp://"] {
        let mut from = 0;
        while let Some(pos) = lower.get(from..).and_then(|rest| rest.find(scheme)) {
            let start = from + pos;
            let url: String = text
                .get(start..)
                .unwrap_or_default()
                .chars()
                .take_while(|c| {
                    !c.is_whitespace() && !matches!(c, '"' | '\'' | ')' | '<' | '>' | ',')
                })
                .collect();
            let url = url.trim_end_matches(['.', ';']).to_owned();
            if url.len() > scheme.len() && !found.contains(&url) {
                found.push(url);
            }
            from = start + scheme.len();
        }
    }
    found
}

fn find_uncs(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut from = 0;
    while let Some(pos) = text.get(from..).and_then(|rest| rest.find("\\\\")) {
        let start = from + pos;
        let path: String = text
            .get(start..)
            .unwrap_or_default()
            .chars()
            .take_while(|c| !c.is_whitespace() && !matches!(c, '"' | '\'' | ')'))
            .collect();
        let host_ok = path.chars().nth(2).is_some_and(char::is_alphanumeric);
        if host_ok && !found.contains(&path) {
            found.push(path);
        }
        from = start + 2;
    }
    found
}

#[cfg(test)]
mod tests;
