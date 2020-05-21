use std::{fs, io, path::Path};

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

pub fn print_erasable(s: &str) {
    print!(
        "{}    \r{}",
        s,
        if cfg!(debug_assertions) { "\n" } else { "" }
    );
    let _ = io::Write::flush(&mut io::stdout());
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
