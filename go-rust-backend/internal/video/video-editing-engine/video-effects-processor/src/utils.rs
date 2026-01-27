use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use crate::types::CPoint;

#[allow(dead_code)]
pub fn export_points_to_csv(filename: &str, points: &[CPoint]) -> std::io::Result<()> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "x,y,timestamp_ms")?;
    for p in points {
        writeln!(writer, "{},{},{}", p.x, p.y, p.timestamp_ms)?;
    }
    Ok(())
}

pub fn ensure_repo_output_dir() -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = base.join("../../../../output");
    if let Err(e) = fs::create_dir_all(&out_dir) {
        eprintln!("[debug] failed to create output dir {:?}: {}", out_dir, e);
    }
    out_dir
}


