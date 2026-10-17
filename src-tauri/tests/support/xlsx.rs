use std::{fs::File, io::Write, path::Path};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

#[derive(Clone)]
pub enum Cell {
    Text(String),
    Number(f64),
    Date(f64),
    Boolean(bool),
    Formula(String, f64),
    Empty,
}
pub fn text(value: &str) -> Cell {
    Cell::Text(value.into())
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn column(mut index: usize) -> String {
    let mut value = String::new();
    loop {
        value.insert(0, (b'A' + (index % 26) as u8) as char);
        if index < 26 {
            return value;
        }
        index = index / 26 - 1;
    }
}
fn entry(zip: &mut ZipWriter<File>, name: &str, data: &str) {
    zip.start_file(
        name,
        SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
    )
    .unwrap();
    zip.write_all(data.as_bytes()).unwrap();
}
pub fn write(path: &Path, sheets: &[(String, Vec<Vec<Cell>>)], date_1904: bool) {
    let mut zip = ZipWriter::new(File::create(path).unwrap());
    let mut content = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>"#,
    );
    let mut workbook = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><workbookPr date1904="{}"/><sheets>"#,
        if date_1904 { 1 } else { 0 }
    );
    let mut relationships = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for (index, (name, rows)) in sheets.iter().enumerate() {
        let number = index + 1;
        content.push_str(&format!(r#"<Override PartName="/xl/worksheets/sheet{number}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#));
        workbook.push_str(&format!(
            r#"<sheet name="{}" sheetId="{number}" r:id="rId{number}"/>"#,
            escape(name)
        ));
        relationships.push_str(&format!(r#"<Relationship Id="rId{number}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{number}.xml"/>"#));
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
        );
        for (row, cells) in rows.iter().enumerate() {
            xml.push_str(&format!(r#"<row r="{}">"#, row + 1));
            for (col, cell) in cells.iter().enumerate() {
                let address = format!("{}{}", column(col), row + 1);
                match cell {
                Cell::Text(value) => xml.push_str(&format!(r#"<c r="{address}" t="inlineStr"><is><t xml:space="preserve">{}</t></is></c>"#, escape(value))),
                Cell::Number(value) => xml.push_str(&format!(r#"<c r="{address}"><v>{value}</v></c>"#)),
                Cell::Date(value) => xml.push_str(&format!(r#"<c r="{address}" s="1"><v>{value}</v></c>"#)),
                Cell::Boolean(value) => xml.push_str(&format!(r#"<c r="{address}" t="b"><v>{}</v></c>"#, if *value { 1 } else { 0 })),
                Cell::Formula(formula, cached) => xml.push_str(&format!(r#"<c r="{address}"><f>{}</f><v>{cached}</v></c>"#, escape(formula))),
                Cell::Empty => {},
            }
            }
            xml.push_str("</row>");
        }
        xml.push_str("</sheetData></worksheet>");
        entry(&mut zip, &format!("xl/worksheets/sheet{number}.xml"), &xml);
    }
    content.push_str("</Types>");
    workbook.push_str("</sheets></workbook>");
    relationships.push_str(r#"<Relationship Id="rIdStyle" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#);
    entry(&mut zip, "[Content_Types].xml", &content);
    entry(&mut zip, "xl/workbook.xml", &workbook);
    entry(&mut zip, "xl/_rels/workbook.xml.rels", &relationships);
    entry(
        &mut zip,
        "_rels/.rels",
        r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
    );
    entry(
        &mut zip,
        "xl/styles.xml",
        r#"<?xml version="1.0" encoding="UTF-8"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="1"><font/></fonts><fills count="1"><fill/></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf numFmtId="0"/></cellStyleXfs><cellXfs count="2"><xf numFmtId="0"/><xf numFmtId="14" applyNumberFormat="1"/></cellXfs></styleSheet>"#,
    );
    zip.finish().unwrap();
}
pub fn family() -> Vec<(String, Vec<Vec<Cell>>)> {
    use bookbuddy_lib::domain::import::*;
    let headers = CATALOGUE_HEADERS
        .iter()
        .map(|header| text(header))
        .collect();
    let mut first = vec![Cell::Empty; 20];
    first[0] = text("Synthetic board book");
    first[1] = text("Example, Author");
    first[2] = text("Board Book");
    first[3] = Cell::Number(2020.0);
    first[4] = text("Read");
    first[5] = text("Yes");
    first[8] = Cell::Boolean(true);
    first[9] = text("Purchased");
    first[10] = Cell::Number(3.0);
    first[11] = Cell::Date(45000.0);
    first[12] = Cell::Date(45001.0);
    first[13] = text("Historical return date");
    first[14] = text("Short");
    first[15] = text("Toddler");
    first[16] = text("No");
    first[17] = text("Shared source note");
    first[18] = text("Clipped PDF label");
    first[19] = text("Yes");
    let mut second = vec![Cell::Empty; 20];
    second[0] = text("Synthetic second book");
    second[1] = text("??");
    second[2] = text("Unknown");
    second[4] = text("Currently Borrowed");
    second[5] = text("No");
    let loan = vec![
        text("Synthetic board book"),
        text("Example, Author"),
        text("Board Book"),
        Cell::Number(2020.0),
        Cell::Date(45000.0),
        text("Historical return date"),
        text("synthetic-source.pdf"),
    ];
    vec![
        (CATALOGUE.into(), vec![headers, first, second]),
        (
            HISTORY.into(),
            vec![
                HISTORY_HEADERS.iter().map(|header| text(header)).collect(),
                loan.clone(),
                loan,
            ],
        ),
        (
            TITLES.into(),
            vec![
                TITLE_HEADERS.iter().map(|header| text(header)).collect(),
                vec![
                    text("Synthetic board book"),
                    text("Example, Author"),
                    text("Clipped PDF label"),
                    text("Synthetic truncation"),
                    text("Synthetic confirmed title"),
                ],
            ],
        ),
        (
            "Dashboard".into(),
            vec![vec![
                text("This is a summary, not a book"),
                Cell::Formula("1+1".into(), 2.0),
            ]],
        ),
        (
            "How to Use".into(),
            vec![vec![text(
                "Dates are historical return proxies, not verified reading dates.",
            )]],
        ),
    ]
}
