# Security policy · Политика безопасности

## Reporting a vulnerability

Please report vulnerabilities **privately** through GitHub's
"Report a vulnerability" button (Security tab → Advisories) of
<https://github.com/team-ph4r0s/arca> (Security tab → Advisories). Do not open public issues for security problems.

Useful reports include: the affected crate and commit, a minimal input file
(or a generator for it), what you observed (crash, hang, wrong verdict,
sandbox escape, attribution bypass) and what you expected.

Particularly valuable:

- any way for a document to cause network, file-system or IPC access;
- any input that makes the X-ray report `Clean` while active or remote content
  is present, or makes the sanitized copy keep it;
- panics, hangs or unbounded memory use in `arca-container` / `arca-xray`;
- escapes from or resource-limit bypasses in `arca-sandbox`;
- anything that makes the host believe a guest result it should not (a sterile copy
  that passes the host's digest and part-name checks but still carries active content,
  a decoder budget bypass in `arca-abi`), and parser differentials between the X-ray and
  the viewer or an office application.

Known limits are listed in [`docs/SECURITY_THREAT_MODEL.md`](docs/SECURITY_THREAT_MODEL.md)
section 4.10; a report on something listed there is still welcome, but please say what is new.

Please do not attach live malware without prior agreement; a description or
a defanged sample is enough to start.

## Сообщение об уязвимости

Сообщайте об уязвимостях **приватно** — кнопкой «Report a vulnerability»
(вкладка Security → Advisories) в репозитории <https://github.com/team-ph4r0s/arca>. Не открывайте
публичные issues по вопросам безопасности.

Особенно ценны: любой способ заставить документ обратиться к сети, файловой
системе или IPC; вход, при котором рентген показывает «Чистый» при наличии
активного или внешнего содержимого или стерильная копия его сохраняет; паники,
зависания и неограниченный расход памяти в `arca-container` и `arca-xray`;
выход из песочницы или обход её лимитов; обман хоста (стерильная копия, которая проходит
проверку хэша и имён частей, но несёт активное содержимое; обход бюджета декодера в
`arca-abi`); расхождения между рентгеном и просмотром или офисной программой.
Известные ограничения перечислены в [§4.10 модели угроз](docs/SECURITY_THREAT_MODEL.md);
сообщение о них тоже полезно, но укажите, что в нём нового.

## Supported versions

| version | security fixes |
|---|---|
| latest release (0.8.x) and `main` | yes |
| older 0.x releases | no — update to the latest release |

Until 1.0 a fix is released as a new version; older releases are not patched. Releases are signed
with one permanent key since 0.3.0; its certificate SHA-256 is published in the README.

## Contact and response time

- Private report: the "Report a vulnerability" button of
  <https://github.com/team-ph4r0s/arca/security/advisories> (the only repository of the project;
  reports to forks or to the old location are not read).
- We acknowledge a report within **5 working days**, give a first assessment within
  **14 days**, and aim to release a fix for High and Critical issues within **30 days**
  and for the rest with the next release. The reporter is credited in the advisory unless
  they ask not to be.
- Advisories are published after the fixed release is out.

## Поддерживаемые версии

| версия | исправления безопасности |
|---|---|
| последний релиз (0.8.x) и `main` | да |
| более старые релизы 0.x | нет — обновитесь до последнего |

До 1.0 исправление выходит новой версией; старые релизы не исправляются. С 0.3.0 релизы
подписаны одним постоянным ключом; SHA-256 его сертификата опубликован в README.

## Контакт и сроки ответа

- Приватное сообщение: кнопка «Report a vulnerability» на
  <https://github.com/team-ph4r0s/arca/security/advisories> (единственный репозиторий проекта;
  сообщения в форки и в старое расположение не читаются).
- Подтверждаем получение в течение **5 рабочих дней**, даём первую оценку в течение **14 дней**,
  исправление уязвимостей High и Critical стараемся выпустить в течение **30 дней**, остальных —
  в ближайшем релизе. Автор сообщения указывается в advisory, если не просит иного.
- Advisory публикуется после выхода исправленного релиза.
Threat model: [`docs/SECURITY_THREAT_MODEL.md`](docs/SECURITY_THREAT_MODEL.md).
