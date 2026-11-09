# SPDX-License-Identifier: AGPL-3.0-only
"""Generates tests/fixtures/deck-torture.pptx: the things real decks do to text.

Standard library only (the package is written part by part), so it runs anywhere:

    python3 tools/fixtures/make_torture_deck.py tests/fixtures/deck-torture.pptx

What is in it, slide by slide:

1. placeholders that take their place and style from the layout and the master, an all-caps
   title, normal autofit;
2. bullets in the symbol fonts the stock templates use (Wingdings "§", "Ø", "ü", "q"), an arrow
   made by autocorrect (`a:sym`), Greek in the Symbol font, numbered lists, a coloured bullet;
3. sideways text (`vert`, `vert270`, `wordArtVert`), rotated and flipped shapes with text, a
   flipped group and a rotated group inside it;
4. a table with sideways header cells and merged cells, right-to-left paragraphs;
5. text in preset shapes, the three anchors, no-wrap text, line spacing.

All text is synthetic. Nothing here is, or contains, malware.
"""
import sys
import zipfile

A = "http://schemas.openxmlformats.org/drawingml/2006/main"
R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
P = "http://schemas.openxmlformats.org/presentationml/2006/main"
NS = f'xmlns:a="{A}" xmlns:r="{R}" xmlns:p="{P}"'
REL = "http://schemas.openxmlformats.org/package/2006/relationships"
OD = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
HEAD = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
GROUP_HEAD = '<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>'
W, H = 12192000, 6858000  # 16:9


def emu(pt):
    return int(round(pt * 12700))


def rels(items):
    body = "".join(
        f'<Relationship Id="{i}" Type="{OD}/{t}" Target="{target}"/>' for i, t, target in items
    )
    return f'{HEAD}<Relationships xmlns="{REL}">{body}</Relationships>'


_next_id = [10]


def nid():
    _next_id[0] += 1
    return _next_id[0]


def xfrm(x, y, w, h, attrs=""):
    return (
        f'<a:xfrm{(" " + attrs) if attrs else ""}><a:off x="{emu(x)}" y="{emu(y)}"/>'
        f'<a:ext cx="{emu(w)}" cy="{emu(h)}"/></a:xfrm>'
    )


def run(text, rpr=""):
    return f'<a:r><a:rPr lang="ru-RU"{rpr}/><a:t>{text}</a:t></a:r>'


def run_x(text, attrs="", kids=""):
    return f'<a:r><a:rPr lang="ru-RU"{attrs}>{kids}</a:rPr><a:t>{text}</a:t></a:r>'


def para(inner, ppr=""):
    return f"<a:p>{ppr}{inner}</a:p>"


def shape(x, y, w, h, body, prst="rect", fill="DCE6F2", line="1F497D", xf="", body_pr="", name="s"):
    i = nid()
    fill_xml = f'<a:solidFill><a:srgbClr val="{fill}"/></a:solidFill>' if fill else "<a:noFill/>"
    line_xml = (
        f'<a:ln w="12700"><a:solidFill><a:srgbClr val="{line}"/></a:solidFill></a:ln>'
        if line
        else "<a:ln><a:noFill/></a:ln>"
    )
    tx = f"<p:txBody><a:bodyPr {body_pr}/><a:lstStyle/>{body}</p:txBody>" if body else ""
    return (
        f'<p:sp><p:nvSpPr><p:cNvPr id="{i}" name="{name} {i}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>'
        f'<p:spPr>{xfrm(x, y, w, h, xf)}<a:prstGeom prst="{prst}"><a:avLst/></a:prstGeom>'
        f"{fill_xml}{line_xml}</p:spPr>{tx}</p:sp>"
    )


def placeholder(kind, idx, body, body_pr=""):
    i = nid()
    idx_attr = f' idx="{idx}"' if idx is not None else ""
    type_attr = f' type="{kind}"' if kind else ""
    return (
        f'<p:sp><p:nvSpPr><p:cNvPr id="{i}" name="ph {i}"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>'
        f"<p:nvPr><p:ph{type_attr}{idx_attr}/></p:nvPr></p:nvSpPr><p:spPr/>"
        f"<p:txBody><a:bodyPr {body_pr}/><a:lstStyle/>{body}</p:txBody></p:sp>"
    )


def group(x, y, w, h, kids, attrs="", child=None):
    cx, cy, cw, ch = child or (x, y, w, h)
    i = nid()
    return (
        f'<p:grpSp><p:nvGrpSpPr><p:cNvPr id="{i}" name="g {i}"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>'
        f'<p:grpSpPr><a:xfrm{(" " + attrs) if attrs else ""}><a:off x="{emu(x)}" y="{emu(y)}"/>'
        f'<a:ext cx="{emu(w)}" cy="{emu(h)}"/><a:chOff x="{emu(cx)}" y="{emu(cy)}"/>'
        f'<a:chExt cx="{emu(cw)}" cy="{emu(ch)}"/></a:xfrm></p:grpSpPr>{kids}</p:grpSp>'
    )


def slide(shapes, layout):
    xml = (
        f"{HEAD}<p:sld {NS}><p:cSld><p:spTree>{GROUP_HEAD}{shapes}</p:spTree></p:cSld>"
        f'<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>'
    )
    return xml, rels([("rId1", "slideLayout", f"../slideLayouts/slideLayout{layout}.xml")])


def lvl(n, mar, ind, size, bullet):
    return (
        f'<a:lvl{n}pPr marL="{mar}" indent="{ind}" algn="l"><a:spcBef><a:spcPts val="600"/></a:spcBef>'
        f'{bullet}<a:defRPr sz="{size}"><a:solidFill><a:schemeClr val="tx1"/></a:solidFill>'
        f'<a:latin typeface="+mn-lt"/></a:defRPr></a:lvl{n}pPr>'
    )


def wing(ch):
    return f'<a:buFont typeface="Wingdings" pitchFamily="2" charset="2"/><a:buChar char="{ch}"/>'


MASTER = (
    f"{HEAD}<p:sldMaster {NS}><p:cSld><p:bg><p:bgRef idx=\"1001\"><a:schemeClr val=\"bg1\"/></p:bgRef></p:bg>"
    f"<p:spTree>{GROUP_HEAD}"
    '<p:sp><p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>'
    f'<p:spPr>{xfrm(48, 22, 864, 80)}</p:spPr><p:txBody><a:bodyPr anchor="ctr"><a:normAutofit/></a:bodyPr><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody></p:sp>'
    '<p:sp><p:nvSpPr><p:cNvPr id="3" name="Body"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr>'
    f'<p:spPr>{xfrm(48, 115, 864, 380)}</p:spPr><p:txBody><a:bodyPr><a:normAutofit/></a:bodyPr><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody></p:sp>'
    "</p:spTree></p:cSld>"
    '<p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>'
    '<p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/><p:sldLayoutId id="2147483650" r:id="rId2"/></p:sldLayoutIdLst>'
    "<p:txStyles><p:titleStyle>"
    '<a:lvl1pPr algn="l"><a:buNone/><a:defRPr sz="3600" b="1" cap="all"><a:solidFill><a:schemeClr val="tx2"/></a:solidFill><a:latin typeface="+mj-lt"/></a:defRPr></a:lvl1pPr>'
    "</p:titleStyle><p:bodyStyle>"
    + lvl(1, 342900, -342900, 2400, wing("§"))
    + lvl(2, 742950, -285750, 2000, wing("Ø"))
    + lvl(3, 1143000, -228600, 1800, wing("ü"))
    + lvl(4, 1600200, -228600, 1600, wing("q"))
    + "</p:bodyStyle><p:otherStyle>"
    '<a:lvl1pPr marL="0" algn="l"><a:defRPr sz="1800"><a:solidFill><a:schemeClr val="tx1"/></a:solidFill><a:latin typeface="+mn-lt"/></a:defRPr></a:lvl1pPr>'
    "</p:otherStyle></p:txStyles></p:sldMaster>"
)

LAYOUT_TITLE = (
    f'{HEAD}<p:sldLayout {NS} type="title"><p:cSld name="Title"><p:spTree>{GROUP_HEAD}'
    '<p:sp><p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="ctrTitle"/></p:nvPr></p:nvSpPr>'
    f'<p:spPr>{xfrm(120, 120, 720, 140)}</p:spPr><p:txBody><a:bodyPr anchor="b"/><a:lstStyle><a:lvl1pPr algn="ctr"><a:defRPr sz="4800"/></a:lvl1pPr></a:lstStyle><a:p><a:endParaRPr/></a:p></p:txBody></p:sp>'
    '<p:sp><p:nvSpPr><p:cNvPr id="3" name="Subtitle"/><p:cNvSpPr/><p:nvPr><p:ph type="subTitle" idx="1"/></p:nvPr></p:nvSpPr>'
    f'<p:spPr>{xfrm(120, 275, 720, 120)}</p:spPr><p:txBody><a:bodyPr/><a:lstStyle><a:lvl1pPr marL="0" indent="0" algn="ctr"><a:buNone/><a:defRPr sz="2400"/></a:lvl1pPr></a:lstStyle><a:p><a:endParaRPr/></a:p></p:txBody></p:sp>'
    "</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"
)

LAYOUT_BODY = (
    f'{HEAD}<p:sldLayout {NS} type="obj"><p:cSld name="Content"><p:spTree>{GROUP_HEAD}'
    '<p:sp><p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr/>'
    "<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody></p:sp>"
    '<p:sp><p:nvSpPr><p:cNvPr id="3" name="Content"/><p:cNvSpPr/><p:nvPr><p:ph idx="1"/></p:nvPr></p:nvSpPr><p:spPr/>'
    "<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody></p:sp>"
    "</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"
)

THEME = (
    f'{HEAD}<a:theme xmlns:a="{A}" name="Torture"><a:themeElements><a:clrScheme name="T">'
    '<a:dk1><a:srgbClr val="1A1A1A"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>'
    '<a:dk2><a:srgbClr val="1F3864"/></a:dk2><a:lt2><a:srgbClr val="EEECE1"/></a:lt2>'
    '<a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2>'
    '<a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4>'
    '<a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6>'
    '<a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme>'
    '<a:fontScheme name="T"><a:majorFont><a:latin typeface="Calibri Light"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont>'
    '<a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme>'
    '<a:fmtScheme name="T"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst>'
    '<a:lnStyleLst><a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst>'
    "<a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst>"
    '<a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst>'
    "</a:fmtScheme></a:themeElements></a:theme>"
)


def cell(text, attrs="", body_pr="", fill=None):
    fill_xml = f'<a:solidFill><a:srgbClr val="{fill}"/></a:solidFill>' if fill else ""
    return (
        f"<a:tc{attrs}><a:txBody><a:bodyPr {body_pr}/><a:lstStyle/>{para(run(text, ' sz=\"1200\"'))}</a:txBody>"
        f"<a:tcPr{(' ' + body_pr) if body_pr else ''}>{fill_xml}</a:tcPr></a:tc>"
    )


def table(x, y, w, h):
    cols = [60, 60, 200, 200, 200]
    grid = "".join(f'<a:gridCol w="{emu(c)}"/>' for c in cols)
    head = (
        f'<a:tr h="{emu(110)}">'
        + cell("Показатель качества", body_pr='vert="vert270"', fill="D9E2F3")
        + cell("Единица измерения", body_pr='vert="vert270"', fill="D9E2F3")
        + cell("Объединённый заголовок трёх столбцов", attrs=' gridSpan="3"', fill="D9E2F3")
        + cell("", attrs=' hMerge="1"')
        + cell("", attrs=' hMerge="1"')
        + "</a:tr>"
    )
    rows = ""
    for r, (a, b) in enumerate([("A", "шт."), ("B", "кг"), ("C", "%")]):
        first = cell(a, attrs=' rowSpan="3"') if r == 0 else cell("", attrs=' vMerge="1"')
        rows += (
            f'<a:tr h="{emu(34)}">{first}{cell(b)}{cell(f"Строка {r + 1}, значение 1")}'
            f'{cell(f"{(r + 1) * 1234.5:.1f}")}{cell("да" if r % 2 == 0 else "нет")}</a:tr>'
        )
    i = nid()
    return (
        f'<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="{i}" name="Table {i}"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>'
        f'<p:xfrm><a:off x="{emu(x)}" y="{emu(y)}"/><a:ext cx="{emu(w)}" cy="{emu(h)}"/></p:xfrm>'
        f'<a:graphic><a:graphicData uri="{A[:-5]}/table"><a:tbl><a:tblPr firstRow="1" bandRow="1"/>'
        f"<a:tblGrid>{grid}</a:tblGrid>{head}{rows}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>"
    )


def build():
    slides = []

    # 1: title slide through the layout.
    slides.append(slide(
        placeholder("ctrTitle", None, para(run("Годовой отчёт о безопасности")))
        + placeholder("subTitle", 1, para(run("Отдел информационной безопасности, 2026")) + para(run("Версия для руководства"))),
        1,
    ))

    # 2: bullets from the master, symbol runs, numbering.
    body = (
        para(run("Первый уровень со значком из Wingdings"))
        + para(run("Второй уровень, стрелка-указатель"), '<a:pPr lvl="1"/>')
        + para(run("Третий уровень, галочка"), '<a:pPr lvl="2"/>')
        + para(run("Четвёртый уровень, квадрат"), '<a:pPr lvl="3"/>')
        + para(
            run("Было ") + run_x("&#xF0E0;", kids='<a:sym typeface="Wingdings" pitchFamily="2" charset="2"/>')
            + run(" стало; угол ") + run_x("a", kids='<a:latin typeface="Symbol"/>') + run(" = 30°, сумма ")
            + run_x("&#xF0E5;", kids='<a:sym typeface="Symbol"/>') + run(" ≤ 100")
        )
        + para(run("Нумерованный пункт"), '<a:pPr marL="457200" indent="-457200"><a:buFont typeface="+mj-lt"/><a:buAutoNum type="arabicPeriod"/></a:pPr>')
        + para(run("Ещё один нумерованный"), '<a:pPr marL="457200" indent="-457200"><a:buFont typeface="+mj-lt"/><a:buAutoNum type="arabicPeriod"/></a:pPr>')
        + para(run("Красный маркер вполовину текста"), '<a:pPr><a:buClr><a:srgbClr val="C00000"/></a:buClr><a:buSzPct val="60000"/><a:buFont typeface="Wingdings"/><a:buChar char="l"/></a:pPr>')
    )
    slides.append(slide(
        placeholder("title", None, para(run("Маркеры и символьные шрифты"))) + placeholder(None, 1, body),
        2,
    ))

    # 3: sideways, rotated and flipped text.
    t = lambda s, sz=1400: para(run(s, f' sz="{sz}"'))
    grp_kids = (
        shape(560, 150, 150, 50, t("В отражённой группе"), fill="FFF2CC", line="BF9000")
        + shape(560, 215, 150, 50, t("Стрелка вправо"), prst="rightArrow", fill="E2F0D9", line="548235")
    )
    inner = group(730, 150, 160, 115, shape(730, 150, 160, 50, t("Вложенная группа"), fill="FCE4D6", line="C55A11"), attrs='rot="1800000"')
    slides.append(slide(
        placeholder("title", None, para(run("Повороты и вертикальный текст")))
        + shape(48, 120, 40, 360, t("Снизу вверх: vert270"), body_pr='vert="vert270" anchor="ctr"')
        + shape(100, 120, 40, 360, t("Сверху вниз: vert"), body_pr='vert="vert" anchor="ctr"')
        + shape(152, 120, 40, 360, t("СТОЛБИК", 1600), body_pr='vert="wordArtVert"')
        + shape(220, 130, 180, 60, t("Поворот на 30°"), xf='rot="1800000"', fill="E2F0D9", line="548235")
        + shape(220, 240, 180, 60, t("Отражение по горизонтали"), prst="rightArrow", xf='flipH="1"', fill="FFF2CC", line="BF9000")
        + shape(220, 330, 180, 60, t("Отражение по вертикали"), prst="rightArrow", xf='flipV="1"', fill="FCE4D6", line="C55A11")
        + shape(220, 420, 180, 60, t("Поворот на 180°"), xf='rot="10800000"', fill="DDEBF7", line="2E75B6")
        + group(540, 140, 370, 140, grp_kids + inner, attrs='flipH="1"')
        + group(540, 320, 370, 140, grp_kids.replace("отражённой", "перевёрнутой"), attrs='flipV="1"', child=(540, 140, 370, 140)),
        2,
    ))

    # 4: a table with sideways headers and merged cells; right-to-left paragraphs.
    rtl = (
        para(run("שלום עולם: טקסט מימין לשמאל", ' sz="1600"'), '<a:pPr algn="r" rtl="1"/>')
        + para(run("مرحبا بالعالم 2026", ' sz="1600"'), '<a:pPr rtl="1"/>')
    )
    slides.append(slide(
        placeholder("title", None, para(run("Таблица и письмо справа налево")))
        + table(48, 120, 720, 212)
        + shape(48, 360, 400, 100, rtl, fill="F2F2F2", line="7F7F7F"),
        2,
    ))

    # 5: text in presets, anchors, no wrap, spacing.
    long = "Длинная строка без переноса, которая выходит за рамку фигуры"
    spaced = (
        para(run("Полуторный интервал, первая строка абзаца, которая переносится на вторую", ' sz="1200"'), '<a:pPr><a:lnSpc><a:spcPct val="150000"/></a:lnSpc></a:pPr>')
        + para(run("Точный интервал 12 пт", ' sz="1200"'), '<a:pPr algn="ctr"><a:lnSpc><a:spcPts val="1200"/></a:lnSpc></a:pPr>')
        + para(run("По правому краю", ' sz="1200" u="sng"'), '<a:pPr algn="r"/>')
    )
    slides.append(slide(
        placeholder("title", None, para(run("Фигуры, привязка и интервалы")))
        + shape(48, 120, 200, 110, t("Текст в эллипсе по центру"), prst="ellipse", body_pr='anchor="ctr"', fill="DDEBF7", line="2E75B6")
        + shape(270, 120, 200, 110, para(run("Сверху", ' sz="1400"'), '<a:pPr algn="ctr"/>'), prst="roundRect", body_pr='anchor="t"')
        + shape(490, 120, 200, 110, para(run("Снизу", ' sz="1400"'), '<a:pPr algn="ctr"/>'), prst="roundRect", body_pr='anchor="b"')
        + shape(710, 120, 200, 110, t("Ромб с текстом"), prst="diamond", body_pr='anchor="ctr"', fill="E2F0D9", line="548235")
        + shape(48, 260, 200, 50, t(long, 1200), body_pr='wrap="none"', fill="FFF2CC", line="BF9000")
        + shape(48, 340, 420, 150, spaced, fill="F2F2F2", line="7F7F7F")
        + shape(490, 260, 420, 230, t("Шеврон", 1600) , prst="chevron", body_pr='anchor="ctr"', fill="FCE4D6", line="C55A11"),
        2,
    ))
    return slides


def main(out):
    slides = build()
    n = len(slides)
    types = (
        f'{HEAD}<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="xml" ContentType="application/xml"/>'
        '<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>'
        '<Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>'
        '<Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>'
        '<Override PartName="/ppt/slideLayouts/slideLayout2.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>'
        '<Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>'
        + "".join(
            f'<Override PartName="/ppt/slides/slide{i + 1}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>'
            for i in range(n)
        )
        + "</Types>"
    )
    pres = (
        f'{HEAD}<p:presentation {NS}><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>'
        "<p:sldIdLst>"
        + "".join(f'<p:sldId id="{256 + i}" r:id="rId{i + 2}"/>' for i in range(n))
        + f'</p:sldIdLst><p:sldSz cx="{W}" cy="{H}"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>'
    )
    pres_rels = rels(
        [("rId1", "slideMaster", "slideMasters/slideMaster1.xml")]
        + [(f"rId{i + 2}", "slide", f"slides/slide{i + 1}.xml") for i in range(n)]
        + [(f"rId{n + 2}", "theme", "theme/theme1.xml")]
    )
    parts = [
        ("[Content_Types].xml", types),
        ("_rels/.rels", rels([("rId1", "officeDocument", "ppt/presentation.xml")])),
        ("ppt/presentation.xml", pres),
        ("ppt/_rels/presentation.xml.rels", pres_rels),
        ("ppt/slideMasters/slideMaster1.xml", MASTER),
        ("ppt/slideMasters/_rels/slideMaster1.xml.rels", rels([
            ("rId1", "slideLayout", "../slideLayouts/slideLayout1.xml"),
            ("rId2", "slideLayout", "../slideLayouts/slideLayout2.xml"),
            ("rId3", "theme", "../theme/theme1.xml"),
        ])),
        ("ppt/slideLayouts/slideLayout1.xml", LAYOUT_TITLE),
        ("ppt/slideLayouts/_rels/slideLayout1.xml.rels", rels([("rId1", "slideMaster", "../slideMasters/slideMaster1.xml")])),
        ("ppt/slideLayouts/slideLayout2.xml", LAYOUT_BODY),
        ("ppt/slideLayouts/_rels/slideLayout2.xml.rels", rels([("rId1", "slideMaster", "../slideMasters/slideMaster1.xml")])),
        ("ppt/theme/theme1.xml", THEME),
    ]
    for i, (xml, rel) in enumerate(slides):
        parts.append((f"ppt/slides/slide{i + 1}.xml", xml))
        parts.append((f"ppt/slides/_rels/slide{i + 1}.xml.rels", rel))
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for name, data in parts:
            info = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data.encode("utf-8"))


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "deck-torture.pptx")
