use std::fmt;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RomInspection {
    pub path: String,
    pub file_size: usize,
    pub prg_banks: u8,
    pub chr_banks: u8,
    pub mapper: u8,
    pub has_trainer: bool,
    pub has_battery: bool,
    pub mirroring: Mirroring,
    pub expected_size: usize,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Mirroring {
    Horizontal,
    Vertical,
}

impl fmt::Display for RomInspection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ROM: {}", self.path)?;
        writeln!(f, "File size: {} bytes", self.file_size)?;
        writeln!(
            f,
            "PRG banks: {} ({}KB)",
            self.prg_banks,
            self.prg_banks as usize * 16
        )?;
        writeln!(
            f,
            "CHR banks: {} ({}KB)",
            self.chr_banks,
            self.chr_banks as usize * 8
        )?;
        writeln!(f, "Mapper: {}", self.mapper)?;
        writeln!(
            f,
            "Battery-backed RAM: {}",
            if self.has_battery {
                "present"
            } else {
                "absent"
            }
        )?;
        writeln!(
            f,
            "Mirroring: {}",
            match self.mirroring {
                Mirroring::Horizontal => "horizontal",
                Mirroring::Vertical => "vertical",
            }
        )?;
        writeln!(
            f,
            "Trainer: {}",
            if self.has_trainer {
                "present"
            } else {
                "absent"
            }
        )?;
        writeln!(f, "Expected size: {} bytes", self.expected_size)?;
        Ok(())
    }
}

pub fn inspect_rom(path: &Path) -> Result<RomInspection> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    if bytes.len() < 16 {
        bail!("ROM too small: {} bytes", bytes.len());
    }

    if &bytes[0..4] != b"NES\x1A" {
        bail!("missing iNES signature NES<EOF>");
    }

    let prg_banks = bytes[4];
    let chr_banks = bytes[5];
    let flags6 = bytes[6];
    let flags7 = bytes[7];
    let has_trainer = (flags6 & 0x04) != 0;
    let has_battery = (flags6 & 0x02) != 0;
    let mapper = (flags6 >> 4) | (flags7 & 0xF0);
    let mirroring = if (flags6 & 0x01) != 0 {
        Mirroring::Vertical
    } else {
        Mirroring::Horizontal
    };

    let trainer_size = if has_trainer { 512usize } else { 0usize };
    let expected_size =
        16 + trainer_size + (prg_banks as usize * 16 * 1024) + (chr_banks as usize * 8 * 1024);
    if bytes.len() != expected_size {
        bail!(
            "file size {} does not match header expectation {}",
            bytes.len(),
            expected_size
        );
    }

    Ok(RomInspection {
        path: path.display().to_string(),
        file_size: bytes.len(),
        prg_banks,
        chr_banks,
        mapper,
        has_trainer,
        has_battery,
        mirroring,
        expected_size,
    })
}

pub fn set_battery_flag(path: &Path, enabled: bool) -> Result<()> {
    let mut bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    if bytes.len() < 16 || &bytes[0..4] != b"NES\x1A" {
        bail!("cannot update battery flag: invalid iNES header");
    }
    if enabled {
        bytes[6] |= 0x02;
    } else {
        bytes[6] &= !0x02;
    }
    fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_rom(flags6: u8) -> Vec<u8> {
        let mut bytes = vec![0; 16 + 16 * 1024 + 8 * 1024];
        bytes[0..4].copy_from_slice(b"NES\x1A");
        bytes[4] = 1;
        bytes[5] = 1;
        bytes[6] = flags6;
        bytes
    }

    #[test]
    fn reports_and_updates_battery_flag() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.nes");
        fs::write(&path, test_rom(0x03)).unwrap();

        assert!(inspect_rom(&path).unwrap().has_battery);
        set_battery_flag(&path, false).unwrap();
        let report = inspect_rom(&path).unwrap();
        assert!(!report.has_battery);
        assert!(matches!(report.mirroring, Mirroring::Vertical));
    }
}
