//! Refreshing `cell_db` from an OpenCelliD export.
//!
//! The table is OpenCelliD's CSV as it comes (`radio,mcc,net,area,cell,unit,lon,lat,range,
//! samples,changeable,created,updated,averageSignal`), so the export of a country
//! (`https://opencellid.org/ocid/downloads?token=…&type=mcc&file=222.csv.gz`) loads as it
//! is: the rows of that country are replaced, the other countries' rows stay.

use std::io::{BufRead, BufReader, Read};

use covercraft_core::{Error, Result, limits};

use crate::SqlSource;

/// The columns written, in the order of OpenCelliD's header.
const COLUMNS: [&str; 14] =
    ["radio", "mcc", "net", "area", "cell", "unit", "lon", "lat", "range", "samples", "changeable", "created", "updated", "averageSignal"];
/// Rows per `INSERT` (14 parameters each: well under SQLite's and MySQL's limits).
const BATCH: usize = 200;
/// Longest line read before the rest is dropped (a hostile file).
const LINE: usize = 4_096;

/// What an import did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CellImport {
    /// Lines read after the header.
    pub read: usize,
    /// Rows kept (the country and networks asked for) and written.
    pub written: usize,
    /// Rows of that country (and networks) deleted first.
    pub deleted: u64,
    /// Lines skipped as malformed.
    pub skipped: usize,
}

/// The file, gunzipped when it starts with gzip's magic bytes.
pub fn open_export(path: &str) -> Result<Box<dyn BufRead>> {
    let mut f = std::fs::File::open(path).map_err(|e| Error::Invalid(format!("{path}: {e}")))?;
    let mut magic = [0u8; 2];
    let n = f.read(&mut magic).map_err(|e| Error::Invalid(format!("{path}: {e}")))?;
    let f = std::fs::File::open(path).map_err(|e| Error::Invalid(format!("{path}: {e}")))?;
    if n == 2 && magic == [0x1f, 0x8b] { Ok(Box::new(BufReader::new(flate2::read::GzDecoder::new(f)))) } else { Ok(Box::new(BufReader::new(f))) }
}

/// Replaces the rows of `mcc` (and of `nets` only, when given) with the export's.
pub fn import(src: &mut SqlSource, reader: impl BufRead, mcc: &str, nets: &[String]) -> Result<CellImport> {
    let mcc = mcc.trim();
    if mcc.is_empty() || !mcc.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::Invalid(format!("mcc must be a number, got {mcc:?}")));
    }
    let nets: Vec<String> = nets.iter().map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect();
    let mut lines = reader.split(b'\n');
    let header = match lines.next() {
        Some(Ok(h)) => String::from_utf8_lossy(&h).to_string(),
        Some(Err(e)) => return Err(Error::Invalid(format!("reading the export: {e}"))),
        None => return Err(Error::Invalid("empty export".into())),
    };
    let names: Vec<String> = header.trim().split(',').map(|c| c.trim().trim_matches('"').to_string()).collect();
    let index: Vec<usize> = COLUMNS
        .iter()
        .map(|c| {
            names
                .iter()
                .position(|n| n.eq_ignore_ascii_case(c))
                .ok_or_else(|| Error::Invalid(format!("not an OpenCelliD export: no column {c} in {header:?}")))
        })
        .collect::<Result<_>>()?;
    let (i_mcc, i_net) = (index[1], index[2]);

    let mut out = CellImport::default();
    if nets.is_empty() {
        out.deleted = src.execute("DELETE FROM `cell_db` WHERE `mcc` = ?", &[mcc])?;
    } else {
        for n in &nets {
            out.deleted = out.deleted.saturating_add(src.execute("DELETE FROM `cell_db` WHERE `mcc` = ? AND `net` = ?", &[mcc, n])?);
        }
    }
    let mut batch: Vec<Vec<String>> = Vec::with_capacity(BATCH);
    for line in lines {
        let line = line.map_err(|e| Error::Invalid(format!("reading the export: {e}")))?;
        let line = String::from_utf8_lossy(line.get(..line.len().min(LINE)).unwrap_or_default());
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        out.read = out.read.saturating_add(1);
        let fields: Vec<&str> = line.split(',').map(|f| f.trim().trim_matches('"')).collect();
        if fields.len() < names.len() {
            out.skipped = out.skipped.saturating_add(1);
            continue;
        }
        if fields.get(i_mcc).copied() != Some(mcc) || (!nets.is_empty() && !fields.get(i_net).is_some_and(|n| nets.iter().any(|w| w == n))) {
            continue;
        }
        batch.push(index.iter().map(|&i| limits::clip(fields.get(i).copied().unwrap_or_default())).collect());
        if batch.len() >= BATCH {
            out.written = out.written.saturating_add(flush(src, &batch)?);
            batch.clear();
        }
    }
    if !batch.is_empty() {
        out.written = out.written.saturating_add(flush(src, &batch)?);
    }
    Ok(out)
}

fn flush(src: &mut SqlSource, rows: &[Vec<String>]) -> Result<usize> {
    let cols: Vec<String> = COLUMNS.iter().map(|c| format!("`{c}`")).collect();
    let marks = format!("({})", vec!["?"; COLUMNS.len()].join(", "));
    let values: Vec<&str> = rows.iter().map(|_| marks.as_str()).collect();
    let params: Vec<&str> = rows.iter().flat_map(|r| r.iter().map(String::as_str)).collect();
    src.execute(&format!("INSERT INTO `cell_db` ({}) VALUES {}", cols.join(", "), values.join(", ")), &params)?;
    Ok(rows.len())
}

/// How many rows the table has and the span of their `updated` stamps.
pub fn summary(src: &mut SqlSource) -> Result<(u64, String, String)> {
    let t = src.query("SELECT COUNT(*), MIN(`updated`), MAX(`updated`) FROM `cell_db`", &[])?;
    let cell = |i: usize| t.rows.first().and_then(|r| r.get(i)).cloned().flatten().unwrap_or_default();
    Ok((cell(0).parse().unwrap_or(0), cell(1), cell(2)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use covercraft_core::source::CoverageSource;
    use std::io::Cursor;

    const HEADER: &str = "radio,mcc,net,area,cell,unit,lon,lat,range,samples,changeable,created,updated,averageSignal\n";

    #[test]
    fn replaces_the_country_and_keeps_the_rest() {
        let mut src = crate::demo_sqlite().unwrap();
        src.execute("INSERT INTO `cell_db` (`radio`,`mcc`,`net`,`area`,`cell`,`lat`,`lon`,`range`,`samples`) VALUES ('LTE','262','1','1','1','52.5','13.4','500','3')", &[]).unwrap();
        let before = summary(&mut src).unwrap().0;
        assert!(before > 1);
        let csv = format!(
            "{HEADER}LTE,222,10,100,12345,0,9.1200,39.2200,1200,25,1,1700000000,1760000000,-80\nNR,222,1,100,67890,0,9.1300,39.2300,800,5,1,1700000000,1760000001,-90\nGSM,262,1,1,2,0,13.4,52.5,500,3,1,1,2,-70\nbroken,line\n"
        );
        let r = import(&mut src, Cursor::new(csv.as_bytes()), "222", &[]).unwrap();
        assert_eq!((r.read, r.written, r.skipped), (4, 2, 1));
        assert!(r.deleted > 0, "the demo city's Italian cells were replaced");
        let (count, _, max) = summary(&mut src).unwrap();
        assert_eq!(count, 3, "two Italian rows plus the German one kept");
        assert_eq!(max, "1760000001");
        let cells = src.cells(39.22, 9.12, 0.05, 50).unwrap();
        assert_eq!(cells.len(), 2);
        assert!(cells.iter().any(|c| c.cell == "12345" && c.range == "1200"));
    }

    #[test]
    fn a_network_filter_touches_only_those_networks() {
        let mut src = crate::demo_sqlite().unwrap();
        let before = summary(&mut src).unwrap().0;
        let csv = format!("{HEADER}LTE,222,10,1,1,0,9.1,39.2,100,1,1,1,2,-80\nLTE,222,88,1,2,0,9.1,39.2,100,1,1,1,2,-80\n");
        let r = import(&mut src, Cursor::new(csv.as_bytes()), "222", &["10".into()]).unwrap();
        assert_eq!(r.written, 1, "only Vodafone's row is loaded");
        let after = summary(&mut src).unwrap().0;
        assert_eq!(after, before - r.deleted as u64 + 1);
    }

    #[test]
    fn rejects_other_files_and_bad_mcc() {
        let mut src = crate::demo_sqlite().unwrap();
        assert!(import(&mut src, Cursor::new(b"a,b,c\n1,2,3\n"), "222", &[]).is_err());
        assert!(import(&mut src, Cursor::new(b""), "222", &[]).is_err());
        assert!(import(&mut src, Cursor::new(HEADER.as_bytes()), "22x", &[]).is_err());
        assert!(import(&mut src, Cursor::new(HEADER.as_bytes()), "222; DROP TABLE cell_db", &[]).is_err());
        let r = import(&mut src, Cursor::new(HEADER.as_bytes()), "222", &[]).unwrap();
        assert_eq!(r.written, 0);
    }

    #[test]
    fn gzip_and_plain_files_open_alike() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("covercraft-cells-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let plain = dir.join("c.csv");
        std::fs::write(&plain, HEADER).unwrap();
        let gz = dir.join("c.csv.gz");
        let mut enc = flate2::write::GzEncoder::new(std::fs::File::create(&gz).unwrap(), flate2::Compression::fast());
        enc.write_all(HEADER.as_bytes()).unwrap();
        enc.finish().unwrap();
        for p in [plain, gz] {
            let mut line = String::new();
            open_export(p.to_str().unwrap()).unwrap().read_line(&mut line).unwrap();
            assert_eq!(line, HEADER);
        }
        assert!(open_export("/nonexistent/x.csv").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
