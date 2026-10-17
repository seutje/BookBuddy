use crate::domain::{import::*, AppError};
use calamine::{Data, DataType, Reader, Xlsx};
use quick_xml::{events::Event, Reader as XmlReader};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Cursor, Read},
    path::Path,
};

pub const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_EXPANDED_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ROW: usize = 30000;
const MAX_COLUMN: usize = 256;
fn invalid() -> AppError {
    AppError::validation("The file could not be read as a supported XLSX workbook. Keep the original file unchanged and check the selected file.")
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn read_bytes(path: &Path) -> Result<Vec<u8>, AppError> {
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xlsx"))
    {
        return Err(AppError::validation("Choose a local .xlsx workbook."));
    }
    let file = File::open(path).map_err(|_| invalid())?;
    if !file.metadata().map_err(|_| invalid())?.is_file() {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(AppError::validation(
            "The workbook exceeds the 32 MiB file limit.",
        ));
    }
    Ok(bytes)
}
fn cell_address(raw: &[u8]) -> bool {
    let mut column = 0usize;
    let mut row = 0usize;
    let mut seen_digit = false;
    for byte in raw {
        if byte.is_ascii_alphabetic() && !seen_digit {
            column = column
                .saturating_mul(26)
                .saturating_add(usize::from(byte.to_ascii_uppercase() - b'A' + 1));
        } else if byte.is_ascii_digit() {
            seen_digit = true;
            row = row
                .saturating_mul(10)
                .saturating_add(usize::from(byte - b'0'));
        } else {
            return false;
        }
    }
    column > 0 && column <= MAX_COLUMN && row > 0 && row <= MAX_ROW
}
fn preflight(bytes: &[u8]) -> Result<bool, AppError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| invalid())?;
    if archive.len() > 5000 {
        return Err(AppError::validation(
            "The workbook contains too many archive entries.",
        ));
    }
    let mut total = 0u64;
    let mut date_1904 = false;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|_| invalid())?;
        total = total.checked_add(entry.size()).ok_or_else(invalid)?;
        if total > MAX_EXPANDED_BYTES {
            return Err(AppError::validation(
                "The workbook exceeds the 128 MiB expanded-data limit.",
            ));
        }
        let is_sheet = entry.name().starts_with("xl/worksheets/") && entry.name().ends_with(".xml");
        if !is_sheet && entry.name() != "xl/workbook.xml" {
            continue;
        }
        let mut xml = Vec::new();
        entry
            .by_ref()
            .take(MAX_EXPANDED_BYTES + 1)
            .read_to_end(&mut xml)
            .map_err(|_| invalid())?;
        if xml.len() as u64 > MAX_EXPANDED_BYTES {
            return Err(invalid());
        }
        let mut reader = XmlReader::from_reader(xml.as_slice());
        let mut implicit_row = 0usize;
        let mut implicit_column = 0usize;
        loop {
            match reader.read_event().map_err(|_| invalid())? {
                Event::Start(element) | Event::Empty(element) => {
                    if is_sheet && element.local_name().as_ref() == "row" {
                        implicit_row = implicit_row.saturating_add(1);
                        implicit_column = 0;
                        for attribute in element.attributes() {
                            let attribute = attribute.map_err(|_| invalid())?;
                            if attribute.key.as_ref() == "r" {
                                implicit_row = attribute.value.parse().map_err(|_| invalid())?;
                            }
                        }
                        if implicit_row == 0 || implicit_row > MAX_ROW {
                            return Err(AppError::validation(
                                "Worksheet row positions exceed the supported limit.",
                            ));
                        }
                    }
                    if is_sheet && matches!(element.local_name().as_ref(), "dimension" | "f") {
                        for attribute in element.attributes() {
                            let attribute = attribute.map_err(|_| invalid())?;
                            if attribute.key.as_ref() == "ref"
                                && !attribute.value.split(':').all(|address| {
                                    cell_address(address.replace('$', "").as_bytes())
                                })
                            {
                                return Err(AppError::validation(
                                    "Worksheet ranges exceed supported bounds.",
                                ));
                            }
                        }
                    }
                    if is_sheet && element.local_name().as_ref() == "c" {
                        implicit_column = implicit_column.saturating_add(1);
                        if implicit_column > MAX_COLUMN {
                            return Err(AppError::validation(
                                "Worksheet columns exceed the supported limit.",
                            ));
                        }
                        for attribute in element.attributes() {
                            let attribute = attribute.map_err(|_| invalid())?;
                            if attribute.key.as_ref() == "r"
                                && !cell_address(attribute.value.as_bytes())
                            {
                                return Err(AppError::validation("Worksheet cells exceed the supported bounds (30000 rows, 256 columns)."));
                            }
                        }
                    }
                    if !is_sheet && element.local_name().as_ref() == "workbookPr" {
                        for attribute in element.attributes() {
                            let attribute = attribute.map_err(|_| invalid())?;
                            if attribute.key.as_ref() == "date1904" {
                                date_1904 = matches!(attribute.value.as_ref(), "1" | "true");
                            }
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
    }
    Ok(date_1904)
}
fn raw_cell(data: &Data, formula: Option<String>) -> RawCell {
    let (kind, value) = match data {
        Data::Empty => ("empty", serde_json::Value::Null),
        Data::String(value) => ("text", json!(value)),
        Data::Int(value) => ("integer", json!(value)),
        Data::Float(value) => ("number", json!(value)),
        Data::Bool(value) => ("boolean", json!(value)),
        Data::DateTime(value) => ("excel_date", json!(value.as_f64())),
        Data::DateTimeIso(value) => ("iso_date", json!(value)),
        Data::DurationIso(value) => ("duration", json!(value)),
        Data::Error(value) => ("error", json!(value.to_string())),
    };
    let calendar_date = if matches!(data, Data::DateTime(_) | Data::DateTimeIso(_)) {
        data.as_date().map(|date| date.to_string())
    } else {
        None
    };
    RawCell {
        kind: kind.into(),
        value,
        calendar_date,
        formula,
    }
}
pub fn inspect_bytes(source_name: &str, bytes: &[u8]) -> Result<WorkbookData, AppError> {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(AppError::validation(
            "The workbook exceeds the 32 MiB file limit.",
        ));
    }
    let date_1904 = preflight(bytes)?;
    let mut workbook = Xlsx::new(Cursor::new(bytes)).map_err(|_| invalid())?;
    if workbook.sheet_names().len() > 30 {
        return Err(AppError::validation("The workbook has too many sheets."));
    }
    let mut sheets = Vec::new();
    let mut cell_count = 0usize;
    for name in workbook.sheet_names().to_vec() {
        let range = workbook.worksheet_range(&name).map_err(|_| invalid())?;
        if range.height() > MAX_ROW || range.width() > MAX_COLUMN {
            return Err(AppError::validation(
                "A worksheet exceeds the supported row or column limit.",
            ));
        }
        cell_count = cell_count.saturating_add(range.height().saturating_mul(range.width()));
        if cell_count > 2_000_000 {
            return Err(AppError::validation(
                "The workbook has too many cells to inspect safely.",
            ));
        }
        let formulas = workbook.worksheet_formula(&name).map_err(|_| invalid())?;
        let (start_row, start_column) = range.start().unwrap_or((0, 0));
        let rows = range
            .rows()
            .enumerate()
            .map(|(row_index, row)| {
                row.iter()
                    .enumerate()
                    .map(|(column_index, data)| {
                        let formula = formulas
                            .get_value((
                                start_row + row_index as u32,
                                start_column + column_index as u32,
                            ))
                            .filter(|formula| !formula.is_empty())
                            .cloned();
                        raw_cell(data, formula)
                    })
                    .collect()
            })
            .collect();
        sheets.push(RawSheet {
            name,
            start_row: start_row as usize,
            start_column: start_column as usize,
            rows,
        });
    }
    Ok(WorkbookData {
        source_name: source_name.to_string(),
        source_hash: hash(bytes),
        date_1904,
        sheets,
    })
}
pub fn inspect(path: &Path) -> Result<WorkbookData, AppError> {
    let bytes = read_bytes(path)?;
    inspect_bytes(
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Selected workbook.xlsx"),
        &bytes,
    )
}
