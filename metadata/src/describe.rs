use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use analysis::fft::{get_codec_information, get_format};
use symphonia::core::codecs::CODEC_TYPE_NULL;

use crate::crc::media_crc32;

fn to_unix_path_string(path_buf: PathBuf) -> Option<String> {
    let path = path_buf.as_path();
    path.to_str().map(|path_str| path_str.replace("\\", "/"))
}

#[derive(Debug)]
pub struct FileDescription {
    pub root_path: PathBuf,
    pub rel_path: PathBuf,
    pub full_path: PathBuf,
    pub file_name: String,
    pub directory: String,
    pub extension: String,
    pub file_hash: Option<String>,
    pub last_modified: String,
}

impl FileDescription {
    pub fn get_crc(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let full_path = self.root_path.join(&self.directory).join(&self.file_name);

        if self.file_hash.is_none() {
            let file = File::open(&full_path)?;
            let mut reader = BufReader::new(file);
            let mut buffer = vec![0; CHUNK_SIZE];
            let mut crc: u32 = 0;

            loop {
                let bytes_read = reader.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                crc = media_crc32(&buffer, crc, 0, bytes_read);
            }

            let result = format!("{:08x}", crc);
            self.file_hash = Some(result.clone());
            Ok(result)
        } else {
            Ok(self.file_hash.clone().unwrap())
        }
    }

    pub fn get_codec_information(&mut self) -> Result<(u32, f64), symphonia::core::errors::Error> {
        let format =
            get_format(self.full_path.to_str().unwrap()).expect("no supported audio tracks");
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .expect("No supported audio tracks");

        get_codec_information(track)
    }
}

const CHUNK_SIZE: usize = 1024 * 400;

pub fn describe_file(
    file_path: &Path,
    lib_path: &Path,
) -> Result<FileDescription, Box<dyn std::error::Error>> {
    // Get file name
    let file_name = file_path
        .file_name()
        .and_then(OsStr::to_str)
        .map(String::from)
        .ok_or("Failed to get file name")?;

    let rel_path = file_path.strip_prefix(lib_path)?;

    // Get directory
    let directory = to_unix_path_string(
        rel_path
            .parent()
            .and_then(Path::to_str)
            .map(String::from)
            .unwrap_or_else(|| String::from(""))
            .into(),
    )
    .unwrap();

    // Get file extension
    let extension = file_path
        .extension()
        .and_then(OsStr::to_str)
        .map(String::from)
        .unwrap_or_else(|| String::from(""));

    // Get last modified time
    let metadata = file_path.metadata()?;
    let last_modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
    let last_modified = format!("{}", last_modified);

    Ok(FileDescription {
        root_path: lib_path.to_path_buf(),
        rel_path: rel_path.to_path_buf(),
        full_path: file_path.to_path_buf(),
        file_name,
        directory,
        extension,
        file_hash: None,
        last_modified,
    })
}

// Helper function to format errors
impl fmt::Display for FileDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FileDescription {{ file_name: {}, directory: {}, extension: {}, last_modified: {} }}",
            self.file_name, self.directory, self.extension, self.last_modified
        )
    }
}
