use std::path::{Path, PathBuf};
use lnk::ShellLink;
use pelite::{pe32, pe64, ImageMap};

#[derive(Debug, Clone)]
pub enum Icon {
    Path(PathBuf),
    Image(Vec<u8>),
}

pub(crate) fn read_taskbar_elements() -> Option<Vec<Shortcut>> {
    let home_dir = dirs::home_dir()?;

    let taskbar_dir = home_dir
        .join(r"AppData\Roaming\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar");

    if !taskbar_dir.exists() {
        eprintln!("Taskbar directory not found");
        return None;
    };

    let entries = taskbar_dir.read_dir();

    if entries.is_err() {
        eprintln!("Failed to read taskbar directory");
        return None;
    }

    let entries = entries.unwrap();

    let shortcuts = entries
        .filter(|e| e.is_ok())
        .map(|e| e.unwrap().path())
        .filter(|path| path.is_file())
        .filter(|path| path.extension().unwrap_or_default() == "lnk")
        .map(Shortcut::from)
        .collect::<Vec<_>>();

    Some(shortcuts)
}

#[derive(Debug, Clone)]
pub struct Shortcut {
    pub name: String,
    pub target: PathBuf,
    pub icon_path: Option<Icon>,
    pub icon_index: i32,
    pub args: String,
}

impl Shortcut {
    fn get_path(lnk: &ShellLink, name: &str) -> Option<PathBuf> {
        if name == "File Explorer" {
            return Some(PathBuf::from(r"C:\Windows\explorer.exe"));
        }

        match lnk.link_target() {
            Some(target) => Some(PathBuf::from(target)),
            None => {
                let start = PathBuf::from(
                    lnk.string_data()
                        .working_dir()
                        .to_owned()
                        .unwrap_or_default(),
                );
                let relative_path = lnk.string_data().relative_path().to_owned()?;
                let path = match start.join(&relative_path).canonicalize() {
                    Ok(path) => path,
                    Err(_) => start
                        .parent()
                        .unwrap_or(&PathBuf::from("C:\\"))
                        .join(&relative_path)
                        .canonicalize()
                        .unwrap_or_default(),
                };

                Some(path)
            }
        }
    }

    fn get_icon(lnk: &ShellLink, target: &Path) -> Option<Icon> {
        match lnk.string_data().icon_location() {
            Some(path) => {
                let path = PathBuf::from(path);
                match path.extension().unwrap() != "exe" {
                    true => Some(Icon::Path(path)),
                    false => Shortcut::extract_icon_from_exe(&target),
                }
            }
            None => Shortcut::extract_icon_from_exe(target),
        }
    }

    fn extract_icon_from_exe(path: &Path) -> Option<Icon> {
        let image = ImageMap::open(path).ok()?;

        if let Ok(view) = pe64::PeView::from_bytes(&image) {
            if let Some(icon) = Shortcut::extract_from_view_64(view) {
                return Some(icon);
            }
        }

        if let Ok(view) = pe32::PeView::from_bytes(&image) {
            if let Some(icon) = Shortcut::extract_from_view_32(view) {
                return Some(icon);
            }
        }

        None
    }

    fn extract_from_view_32(view: pe32::PeView) -> Option<Icon> {
        use pe32::Pe;
        let resources = view.resources().ok()?;
        let (_, group) = resources.icons().next()?.ok()?;

        let mut ico_data = Vec::new();
        group.write(&mut ico_data).ok()?;

        Some(Icon::Image(ico_data))
    }

    fn extract_from_view_64(view: pe64::PeView) -> Option<Icon> {
        use pe64::Pe;
        let resources = view.resources().ok()?;
        let (_, group) = resources.icons().next()?.ok()?;

        let mut ico_data = Vec::new();
        group.write(&mut ico_data).ok()?;

        Some(Icon::Image(ico_data))
    }
}

impl From<PathBuf> for Shortcut {
    fn from(path: PathBuf) -> Self {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();

        let lnk = ShellLink::open(&path, lnk::encoding::WINDOWS_1252).unwrap();

        let target = match Shortcut::get_path(&lnk, &name) {
            Some(target) => target,
            None => panic!("Failed to get target for: {}", name),
        };

        let icon_path = Shortcut::get_icon(&lnk, &target);

        let icon_index = lnk.header().icon_index().to_owned();
        let args = lnk
            .string_data()
            .command_line_arguments()
            .to_owned()
            .unwrap_or_default();

        Self {
            name,
            target,
            icon_path,
            icon_index,
            args,
        }
    }
}
