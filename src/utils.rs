use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[macro_export]
macro_rules! colorln {
    ($color:ident, $format:literal $(, $item:expr)* $(,)? ) => {
        println!("{}", colored::Colorize::$color(format!($format, $($item),*).as_str()))
    };
}

#[macro_export]
macro_rules! waitln {
    ($format:literal $(, $item:expr)* $(,)?) => {
        print!($format, $($item),*);
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
}

pub fn capitalize_path(top: &Path, path: &Path) -> PathBuf {
    let diff = pathdiff::diff_paths(path, top).unwrap();
    let capped: PathBuf = diff
        .iter()
        .map(|part| {
            part.to_string_lossy()
                .chars()
                .enumerate()
                .flat_map(|(i, c)| {
                    if i == 0 {
                        c.to_uppercase().collect::<Vec<_>>()
                    } else {
                        vec![c]
                    }
                })
                .collect::<String>()
        })
        .map(PathBuf::from)
        .collect();
    top.join(capped)
}

pub fn remove_path<P, Q>(top: P, name: Q) -> io::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let top = top.as_ref();
    let mut name = name.as_ref();
    // Delete file
    let path = top.join(name);
    if path.is_file() {
        fs::remove_file(path)?;
    }
    // Delete empty folders
    while let Some(parent) = name.parent() {
        if parent.iter().count() == 0 {
            break;
        }
        if fs::remove_dir(top.join(parent)).is_err() {
            break;
        }
        name = parent;
    }
    Ok(())
}

pub fn create_dirs<P>(path: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if path.extension().is_some() {
        fs::create_dir_all(path.parent().unwrap())?;
    } else {
        fs::create_dir_all(path)?;
    }
    Ok(())
}
