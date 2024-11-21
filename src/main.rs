use chrono::offset::Local;
use chrono::DateTime;
use python_input::input;
use std::ffi::{OsStr, OsString};
use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const CLI_NAME: &str = "something";
const DAYS_PER_WEEK: u64 = 7;
const HOURS_PER_DAY: u64 = 24;
const MINUTES_PER_HOUR: u64 = 60;
const SECONDS_PER_MINUTE: u64 = 60;

#[derive(Debug)]
enum Error {
    InvalidArgument { name: String },
    ReadDirError { dirname: OsString },
    ReadDirEntryError,
    ReadFileError,
    TimeSubtractionError,
    DeleteFailed { filename: OsString },
    Cancelled,
}

#[derive(Debug)]
struct Args {
    path: PathBuf,
    duration: Duration,
}

type Result<T> = core::result::Result<T, Error>;

fn get_args<'a>() -> Result<Args> {
    let path_arg = clap::Arg::new("path")
        .short('p')
        .long("path")
        .required(true);

    let duration_arg = clap::Arg::new("duration")
        .short('d')
        .long("duration")
        .required(true);

    let command = clap::Command::new(CLI_NAME).arg(path_arg).arg(duration_arg);
    let matches = command.get_matches();

    let path_str = matches
        .get_raw("path")
        .and_then(Iterator::last)
        .expect("path is required");
    let path = Path::new(path_str).to_owned();

    let duration = matches
        .get_raw("duration")
        .and_then(Iterator::last)
        .and_then(OsStr::to_str)
        .expect("duration is required");

    let duration = match duration.chars().last() {
        Some('d') => {
            let number_of_days: u64 =
                duration
                    .trim_end_matches('d')
                    .parse()
                    .or(Err(Error::InvalidArgument {
                        name: "duration".to_string(),
                    }))?;

            Duration::from_secs(
                number_of_days * SECONDS_PER_MINUTE * MINUTES_PER_HOUR * HOURS_PER_DAY,
            )
        }

        Some('w') => {
            let number_of_weeks: u64 =
                duration
                    .trim_end_matches('w')
                    .parse()
                    .or(Err(Error::InvalidArgument {
                        name: "duration".to_string(),
                    }))?;

            Duration::from_secs(
                number_of_weeks
                    * SECONDS_PER_MINUTE
                    * MINUTES_PER_HOUR
                    * HOURS_PER_DAY
                    * DAYS_PER_WEEK,
            )
        }
        _ => Err(Error::InvalidArgument {
            name: "duration".to_string(),
        })?,
    };

    Ok(Args { path, duration })
}

fn main_script() -> Result<usize> {
    let Args { path, duration } = get_args()?;

    let threshold_time = SystemTime::now()
        .checked_sub(duration)
        .ok_or(Error::TimeSubtractionError)?;
    let threshold_time_datetime: DateTime<Local> = threshold_time.into();
    let threshold_time_str = threshold_time_datetime.format("%d/%m/%Y %T");
    let path_str = path.clone().into_os_string();

    let answer = input(&format!(
        "Removing all files older than {threshold_time_str} in {path_str:?}. Enter \"y\" to confirm. "
    ));
    if answer != "y" {
        Err(Error::Cancelled)?
    }
    let file_is_old_mapping: Result<Vec<(DirEntry, bool)>> = fs::read_dir(path)
        .or(Err(Error::ReadDirError { dirname: path_str }))?
        .map(|v| -> Result<(DirEntry, bool)> {
            let file = v.or(Err(Error::ReadDirEntryError))?;
            let modified = file
                .metadata()
                .or(Err(Error::ReadFileError))?
                .modified()
                .or(Err(Error::ReadFileError))?;
            let is_old = modified.lt(&threshold_time);
            Ok((file, is_old))
        })
        .collect();
    let file_is_old_mapping = file_is_old_mapping?;

    let results: Result<Vec<()>> = file_is_old_mapping
        .iter()
        .filter_map(|(file, is_old)| if *is_old { Some(file) } else { None })
        .map(|file| {
            let path = file.path();
            if path.is_file() {
                fs::remove_file(file.path()).or(Err(Error::DeleteFailed {
                    filename: file.file_name(),
                }))
            } else if path.is_dir() {
                fs::remove_dir_all(file.path()).or(Err(Error::DeleteFailed {
                    filename: file.file_name(),
                }))
            } else {
                Err(Error::DeleteFailed {
                    filename: file.file_name(),
                })
            }
        })
        .collect();

    Ok(results?.len())
}

fn main() {
    let err = match main_script() {
        Ok(files_count) => {
            println!("Successfully removed {files_count} files!");
            return;
        }
        Err(err) => err,
    };

    match err {
        Error::InvalidArgument { name } => panic!("Invalid argument provided for argument {name}"),
        Error::ReadDirError { dirname } => panic!("Failed to read directory {dirname:?}"),
        Error::ReadFileError => panic!("Failed to read file"),
        Error::TimeSubtractionError => panic!("Failed to subtract time"),
        Error::DeleteFailed { filename } => panic!("Failed to delete {filename:?}"),
        Error::ReadDirEntryError => panic!("Failed to read dir entry"),
        Error::Cancelled => panic!("Cancelled by user."),
    }
}
