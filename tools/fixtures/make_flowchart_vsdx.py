# SPDX-License-Identifier: AGPL-3.0-only
"""Builds tests/fixtures/flowchart.vsdx: a drawing laid out the way Visio itself writes one.

`drawing.vsdx` (make_visio.py) is a minimal package for the reading view. This one is for the
page drawing: masters with their own geometry, instances that override only the cells whose
results differ (`F="Inh"`), style sheets, a theme, connectors with arrowheads glued to shapes,
a rounded rectangle, a dashed line, a rotated shape, a flipped shape, a group, a hidden layer
and a picture stored in `visio/media`. Standard library only; the output is deterministic.

    python3 tools/fixtures/make_flowchart_vsdx.py tests/fixtures/flowchart.vsdx

All text is synthetic. Nothing here is, or contains, malware.
"""
import struct
import sys
import zipfile
import zlib

NS = (
    'xmlns="http://schemas.microsoft.com/office/visio/2012/main" '
    'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xml:space="preserve"'
)
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
VREL = "http://schemas.microsoft.com/visio/2010/relationships"
HEAD = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'


def rels(items):
    body = "".join(f'<Relationship Id="{i}" Type="{t}" Target="{g}"/>' for i, t, g in items)
    return (
        f'{HEAD}<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        f"{body}</Relationships>"
    )


def cell(n, v, f=None, u=None):
    extra = (f' U="{u}"' if u else "") + (f' F="{f}"' if f else "")
    return f'<Cell N="{n}" V="{v}"{extra}/>'


def row(t, ix, **cells):
    return f'<Row T="{t}" IX="{ix}">' + "".join(cell(k, v) for k, v in cells.items()) + "</Row>"


def geometry(ix, rows, no_fill=0):
    return (
        f'<Section N="Geometry" IX="{ix}">{cell("NoFill", no_fill)}{cell("NoLine", 0)}'
        + "".join(rows)
        + "</Section>"
    )


def png(width, height, rgb):
    """A flat-colour PNG with a darker frame."""
    raw = b""
    for y in range(height):
        raw += b"\x00"
        for x in range(width):
            edge = x < 4 or y < 4 or x >= width - 4 or y >= height - 4
            raw += bytes(c // 2 for c in rgb) if edge else bytes(rgb)

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


CONTENT_TYPES = (
    f'{HEAD}<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
    '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
    '<Default Extension="xml" ContentType="application/xml"/>'
    '<Default Extension="png" ContentType="image/png"/>'
    '<Override PartName="/visio/document.xml" ContentType="application/vnd.ms-visio.drawing.main+xml"/>'
    '<Override PartName="/visio/pages/pages.xml" ContentType="application/vnd.ms-visio.pages+xml"/>'
    '<Override PartName="/visio/pages/page1.xml" ContentType="application/vnd.ms-visio.page+xml"/>'
    '<Override PartName="/visio/masters/masters.xml" ContentType="application/vnd.ms-visio.masters+xml"/>'
    + "".join(
        f'<Override PartName="/visio/masters/master{i}.xml" ContentType="application/vnd.ms-visio.master+xml"/>'
        for i in range(1, 6)
    )
    + '<Override PartName="/visio/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>'
    "</Types>"
)

# Style sheets as Visio writes them: 0 is the root every style inherits from.
DOCUMENT = (
    f"{HEAD}<VisioDocument {NS}>"
    '<Colors><ColorEntry IX="0" RGB="#000000"/><ColorEntry IX="1" RGB="#FFFFFF"/>'
    '<ColorEntry IX="2" RGB="#FF0000"/><ColorEntry IX="3" RGB="#00FF00"/><ColorEntry IX="4" RGB="#0000FF"/></Colors>'
    "<StyleSheets>"
    '<StyleSheet ID="0" NameU="No Style" Name="No Style">'
    + cell("LineWeight", "0.01041666666666667")
    + cell("LineColor", "0")
    + cell("LinePattern", "1")
    + cell("Rounding", "0")
    + cell("BeginArrow", "0")
    + cell("EndArrow", "0")
    + cell("EndArrowSize", "2")
    + cell("FillForegnd", "1")
    + cell("FillBkgnd", "0")
    + cell("FillPattern", "1")
    + cell("VerticalAlign", "1")
    + '<Section N="Character"><Row IX="0">'
    + cell("Font", "Calibri")
    + cell("Color", "0")
    + cell("Style", "0")
    + cell("Size", "0.1666666666666667")
    + "</Row></Section>"
    + '<Section N="Paragraph"><Row IX="0">'
    + cell("HorzAlign", "1")
    + "</Row></Section></StyleSheet>"
    '<StyleSheet ID="3" NameU="Normal" Name="Normal" LineStyle="0" FillStyle="0" TextStyle="0">'
    + cell("LineWeight", "0.01388888888888889")
    + cell("LineColor", "#1F3864")
    + "</StyleSheet>"
    '<StyleSheet ID="7" NameU="Connector" Name="Connector" LineStyle="3" FillStyle="3" TextStyle="3">'
    + cell("EndArrow", "4")
    + cell("LineColor", "#404040")
    + '<Section N="Character"><Row IX="0">'
    + cell("Size", "0.1111111111111111")
    + "</Row></Section></StyleSheet>"
    "</StyleSheets></VisioDocument>"
)

THEME = (
    f'{HEAD}<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office">'
    '<a:themeElements><a:clrScheme name="Office"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>'
    '<a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>'
    '<a:accent1><a:srgbClr val="5B9BD5"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2>'
    '<a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4>'
    '<a:accent5><a:srgbClr val="4472C4"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6>'
    '<a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme>'
    "</a:themeElements></a:theme>"
)

MASTERS = (
    f"{HEAD}<Masters {NS}>"
    + "".join(
        f'<Master ID="{i}" NameU="{name}" Name="{name}" UniqueID="{{0000000{i}-0000-0000-0000-000000000000}}">'
        f'<PageSheet LineStyle="0" FillStyle="0" TextStyle="0">{cell("PageWidth", "1")}{cell("PageHeight", "1")}</PageSheet>'
        f'<Rel r:id="rId{i}"/></Master>'
        for i, name in [(1, "Process"), (2, "Decision"), (3, "Start/End"), (4, "Dynamic connector"), (5, "Database")]
    )
    + "</Masters>"
)


def master(shape):
    return f"{HEAD}<MasterContents {NS}><Shapes>{shape}</Shapes></MasterContents>"


def box_cells(w, h):
    return (
        cell("PinX", w / 2) + cell("PinY", h / 2) + cell("Width", w) + cell("Height", h)
        + cell("LocPinX", w / 2, "Width*0.5") + cell("LocPinY", h / 2, "Height*0.5")
        + cell("Angle", 0) + cell("FlipX", 0) + cell("FlipY", 0)
    )


# Process: a rectangle in relative rows (it follows the size of its instance by itself).
M_PROCESS = master(
    '<Shape ID="5" Type="Shape" LineStyle="3" FillStyle="3" TextStyle="3">'
    + box_cells(1.5, 1.0)
    + cell("FillForegnd", "#5B9BD5")
    + '<Section N="Character"><Row IX="0">' + cell("Color", "#FFFFFF") + "</Row></Section>"
    + geometry(0, [
        row("RelMoveTo", 1, X=0, Y=0), row("RelLineTo", 2, X=1, Y=0),
        row("RelLineTo", 3, X=1, Y=1), row("RelLineTo", 4, X=0, Y=1), row("RelLineTo", 5, X=0, Y=0),
    ])
    + "</Shape>"
)

# Decision: a diamond in absolute rows whose values are those of the master's own size; an
# instance of another size states the cells whose results differ.
M_DECISION = master(
    '<Shape ID="5" Type="Shape" LineStyle="3" FillStyle="3" TextStyle="3">'
    + box_cells(1.5, 1.0)
    + cell("FillForegnd", "#ED7D31")
    + '<Section N="Character"><Row IX="0">' + cell("Color", "#FFFFFF") + "</Row></Section>"
    + geometry(0, [
        row("MoveTo", 1, X=0, Y=0.5), row("LineTo", 2, X=0.75, Y=0),
        row("LineTo", 3, X=1.5, Y=0.5), row("LineTo", 4, X=0.75, Y=1), row("LineTo", 5, X=0, Y=0.5),
    ])
    + "</Shape>"
)

# Start/End: a rectangle whose corners the Rounding cell rounds.
M_TERMINATOR = master(
    '<Shape ID="5" Type="Shape" LineStyle="3" FillStyle="3" TextStyle="3">'
    + box_cells(1.5, 0.6)
    + cell("FillForegnd", "#70AD47")
    + cell("Rounding", "0.3")
    + '<Section N="Character"><Row IX="0">' + cell("Color", "#FFFFFF") + "</Row></Section>"
    + geometry(0, [
        row("RelMoveTo", 1, X=0, Y=0), row("RelLineTo", 2, X=1, Y=0),
        row("RelLineTo", 3, X=1, Y=1), row("RelLineTo", 4, X=0, Y=1), row("RelLineTo", 5, X=0, Y=0),
    ])
    + "</Shape>"
)

# Dynamic connector: a 1-D shape; each instance carries its own route.
M_CONNECTOR = master(
    '<Shape ID="5" Type="Shape" LineStyle="7" FillStyle="7" TextStyle="7">'
    + cell("PinX", 0.5) + cell("PinY", 0.5) + cell("Width", 1) + cell("Height", 1)
    + cell("LocPinX", 0.5) + cell("LocPinY", 0.5)
    + cell("BeginX", 0) + cell("BeginY", 0) + cell("EndX", 1) + cell("EndY", 1)
    + cell("ObjType", 2)
    + geometry(0, [row("MoveTo", 1, X=0, Y=0), row("LineTo", 2, X=1, Y=1)], no_fill=1)
    + "</Shape>"
)

# Database: a cylinder, a body with elliptical arcs and a lid drawn by a second section.
M_DATABASE = master(
    '<Shape ID="5" Type="Shape" LineStyle="3" FillStyle="3" TextStyle="3">'
    + box_cells(1.0, 1.2)
    + cell("FillForegnd", "#FFC000")
    + geometry(0, [
        row("MoveTo", 1, X=0, Y=1.05),
        row("LineTo", 2, X=0, Y=0.15),
        row("EllipticalArcTo", 3, X=1, Y=0.15, A=0.5, B=0, C=0, D=3.3333),
        row("LineTo", 4, X=1, Y=1.05),
        row("EllipticalArcTo", 5, X=0, Y=1.05, A=0.5, B=1.2, C=0, D=3.3333),
    ])
    + geometry(1, [
        row("MoveTo", 1, X=0, Y=1.05),
        row("EllipticalArcTo", 2, X=1, Y=1.05, A=0.5, B=0.9, C=0, D=3.3333),
    ], no_fill=1)
    + "</Shape>"
)


def instance(sid, name, master_id, x, y, w=None, h=None, text="", extra="", geom=""):
    size = ""
    if w is not None:
        size = (
            cell("Width", w) + cell("Height", h)
            + cell("LocPinX", w / 2, "Inh") + cell("LocPinY", h / 2, "Inh")
        )
    body = f"<Text>{text}</Text>" if text else ""
    return (
        f'<Shape ID="{sid}" NameU="{name}" Name="{name}" Type="Shape" Master="{master_id}">'
        + cell("PinX", x) + cell("PinY", y) + size + extra + geom + body + "</Shape>"
    )


def connector(sid, x0, y0, x1, y1, text="", extra="", points=None):
    """A connector from (x0, y0) to (x1, y1); `points` are the corners of its route."""
    w, h = x1 - x0, y1 - y0
    pin = ((x0 + x1) / 2, (y0 + y1) / 2)
    route = points or [(x1, y1)]
    rows = [row("MoveTo", 1, X=0, Y=0)]
    for i, (px, py) in enumerate(route):
        rows.append(row("LineTo", i + 2, X=px - x0, Y=py - y0))
    # Visio keeps the local box of a 1-D shape `Width × Height` with the begin point at the origin
    # when neither is flipped; a zero height is written as it is.
    return (
        f'<Shape ID="{sid}" NameU="Dynamic connector" Name="Dynamic connector" Type="Shape" Master="4">'
        + cell("PinX", pin[0], "Inh") + cell("PinY", pin[1], "Inh")
        + cell("Width", w, "Inh") + cell("Height", h, "Inh")
        + cell("LocPinX", w / 2, "Inh") + cell("LocPinY", h / 2, "Inh")
        + cell("BeginX", x0) + cell("BeginY", y0) + cell("EndX", x1) + cell("EndY", y1)
        + cell("TxtPinX", w / 2, "Inh") + cell("TxtPinY", h / 2, "Inh")
        + extra
        + geometry(0, rows, no_fill=1)
        + (f"<Text>{text}</Text>" if text else "")
        + "</Shape>"
    )


def page():
    shapes = (
        instance(1, "Start", 3, 4.25, 10.0, text="Начало")
        + instance(2, "Receive", 1, 4.25, 8.6, 2.2, 0.9, text="Принять заказ и проверить данные клиента")
        # The diamond is twice the master's size: the cells whose results differ are stated.
        + instance(
            3, "Paid", 2, 4.25, 6.8, 2.4, 1.4, text="Оплачен?",
            geom=geometry(0, [
                row("MoveTo", 1, X=0, Y=0.7), row("LineTo", 2, X=1.2, Y=0),
                row("LineTo", 3, X=2.4, Y=0.7), row("LineTo", 4, X=1.2, Y=1.4), row("LineTo", 5, X=0, Y=0.7),
            ]),
        )
        + instance(4, "Ship", 1, 4.25, 4.9, 2.2, 0.9, text="Отгрузить со склада")
        + instance(5, "Remind", 1, 7.2, 6.8, 1.6, 0.9, text="Напомнить",
                   extra=cell("FillForegnd", "#C00000") + cell("LinePattern", 2) + cell("LineWeight", "0.03"))
        + instance(6, "Stock", 5, 1.4, 4.9, text="Склад")
        + instance(7, "End", 3, 4.25, 3.4, text="Конец")
        # Rotated by 30 degrees, and one flipped left to right.
        + instance(8, "Note", 1, 7.0, 9.4, 1.8, 0.7, text="Повёрнутая заметка",
                   extra=cell("Angle", "0.5235987755982988") + cell("FillForegnd", "#7030A0"))
        + instance(9, "Flipped", 2, 1.4, 8.6, 1.6, 1.0, text="Отражённая",
                   extra=cell("FlipX", 1),
                   geom=geometry(0, [
                       row("MoveTo", 1, X=0, Y=0.5), row("LineTo", 2, X=0.8, Y=0),
                       row("LineTo", 3, X=1.6, Y=0.5), row("LineTo", 4, X=0.8, Y=1), row("LineTo", 5, X=0, Y=0.5),
                   ]))
        # A group: its members are placed in the group's own coordinates.
        + '<Shape ID="10" NameU="Legend" Name="Legend" Type="Group" LineStyle="3" FillStyle="3" TextStyle="3">'
        + cell("PinX", 7.2) + cell("PinY", 4.0) + cell("Width", 2.0) + cell("Height", 1.6)
        + cell("LocPinX", 1.0) + cell("LocPinY", 0.8)
        + "<Shapes>"
        + instance(11, "Legend process", 1, 1.0, 1.2, 1.6, 0.5, text="Действие")
        + instance(12, "Legend decision", 2, 1.0, 0.45, 1.6, 0.7, text="Условие",
                   geom=geometry(0, [
                       row("MoveTo", 1, X=0, Y=0.35), row("LineTo", 2, X=0.8, Y=0),
                       row("LineTo", 3, X=1.6, Y=0.35), row("LineTo", 4, X=0.8, Y=0.7), row("LineTo", 5, X=0, Y=0.35),
                   ]))
        + "</Shapes></Shape>"
        # A picture.
        + '<Shape ID="13" NameU="Logo" Name="Logo" Type="Foreign" LineStyle="3" FillStyle="3" TextStyle="3">'
        + cell("PinX", 1.4) + cell("PinY", 10.0) + cell("Width", 1.2) + cell("Height", 0.8)
        + cell("LocPinX", 0.6) + cell("LocPinY", 0.4) + cell("ImgOffsetX", 0) + cell("ImgOffsetY", 0)
        + cell("ImgWidth", 1.2) + cell("ImgHeight", 0.8)
        + '<ForeignData ForeignType="Bitmap" CompressionType="PNG"><Rel r:id="rId1"/></ForeignData></Shape>'
        # A shape on a layer that is switched off.
        + instance(14, "Draft", 1, 1.4, 6.8, 1.6, 0.9, text="Черновик (скрытый слой)", extra=cell("LayerMember", "1"))
        # Connectors.
        + connector(20, 4.25, 9.7, 4.25, 9.05)
        + connector(21, 4.25, 8.15, 4.25, 7.5)
        + connector(22, 4.25, 6.1, 4.25, 5.35, text="да")
        + connector(23, 5.45, 6.8, 6.4, 6.8, text="нет")
        + connector(24, 7.2, 7.25, 5.35, 8.6, points=[(7.2, 8.6), (5.35, 8.6)], extra=cell("LinePattern", 2))
        + connector(25, 4.25, 4.45, 4.25, 3.7)
        + connector(26, 1.9, 4.9, 3.15, 4.9, extra=cell("BeginArrow", 4))
    )
    connects = "".join(
        f'<Connect FromSheet="{c}" FromCell="{fc}" FromPart="{fp}" ToSheet="{s}" ToCell="PinX" ToPart="3"/>'
        for c, fc, fp, s in [
            (20, "BeginX", 9, 1), (20, "EndX", 12, 2), (21, "BeginX", 9, 2), (21, "EndX", 12, 3),
            (22, "BeginX", 9, 3), (22, "EndX", 12, 4), (23, "BeginX", 9, 3), (23, "EndX", 12, 5),
            (24, "BeginX", 9, 5), (24, "EndX", 12, 2), (25, "BeginX", 9, 4), (25, "EndX", 12, 7),
            (26, "BeginX", 9, 6), (26, "EndX", 12, 4),
        ]
    )
    return f"{HEAD}<PageContents {NS}><Shapes>{shapes}</Shapes><Connects>{connects}</Connects></PageContents>"


PAGES = (
    f"{HEAD}<Pages {NS}>"
    '<Page ID="0" NameU="Order" Name="Обработка заказа" ViewScale="1" ViewCenterX="4.25" ViewCenterY="5.5">'
    '<PageSheet LineStyle="0" FillStyle="0" TextStyle="0">'
    + cell("PageWidth", "8.5") + cell("PageHeight", "11") + cell("PageScale", "1") + cell("DrawingScale", "1")
    + '<Section N="Layer"><Row IX="0">' + cell("Name", "Flowchart") + cell("Visible", 1) + cell("Print", 1) + "</Row>"
    + '<Row IX="1">' + cell("Name", "Draft") + cell("Visible", 0) + cell("Print", 0) + "</Row></Section>"
    + '</PageSheet><Rel r:id="rId1"/></Page></Pages>'
)


def main(out):
    parts = [
        ("[Content_Types].xml", CONTENT_TYPES),
        ("_rels/.rels", rels([("rId1", f"{VREL}/document", "visio/document.xml")])),
        ("visio/document.xml", DOCUMENT),
        ("visio/_rels/document.xml.rels", rels([
            ("rId1", f"{VREL}/masters", "masters/masters.xml"),
            ("rId2", f"{VREL}/pages", "pages/pages.xml"),
            ("rId3", f"{REL}/theme", "theme/theme1.xml"),
        ])),
        ("visio/theme/theme1.xml", THEME),
        ("visio/masters/masters.xml", MASTERS),
        ("visio/masters/_rels/masters.xml.rels", rels(
            [(f"rId{i}", f"{VREL}/master", f"master{i}.xml") for i in range(1, 6)]
        )),
        ("visio/masters/master1.xml", M_PROCESS),
        ("visio/masters/master2.xml", M_DECISION),
        ("visio/masters/master3.xml", M_TERMINATOR),
        ("visio/masters/master4.xml", M_CONNECTOR),
        ("visio/masters/master5.xml", M_DATABASE),
        ("visio/pages/pages.xml", PAGES),
        ("visio/pages/_rels/pages.xml.rels", rels([("rId1", f"{VREL}/page", "page1.xml")])),
        ("visio/pages/page1.xml", page()),
        ("visio/pages/_rels/page1.xml.rels", rels(
            [("rId1", f"{REL}/image", "../media/image1.png")]
            + [(f"rId{i + 1}", f"{VREL}/master", f"../masters/master{i}.xml") for i in range(1, 6)]
        )),
    ]
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for name, data in parts:
            info = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data.encode("utf-8"))
        info = zipfile.ZipInfo("visio/media/image1.png", date_time=(2026, 1, 1, 0, 0, 0))
        z.writestr(info, png(96, 64, (0x2E, 0x75, 0xB6)))


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "flowchart.vsdx")
