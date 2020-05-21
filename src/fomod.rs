#![allow(dead_code, unused_variables)]

use std::{
    borrow::Cow,
    fs,
    io::{stdin, stdout, BufRead, Read, Write},
    path::{Path, PathBuf},
};

use serde_derive::Deserialize;
use xmltree::Element;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("XML error: {0}")]
    XML(#[from] xmltree::ParseError),
    #[error("Missing element {0:?}")]
    Missing(String),
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum GroupType {
    SelectAny,
    SelectAll,
    SelectExactlyOne,
    SelectAtMostOne,
    SelectAtLeastOne,
}

#[derive(Debug)]
pub struct Plugin {
    pub name: String,
    pub description: String,
    pub files: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct Group {
    pub name: String,
    pub ty: GroupType,
    pub plugins: Vec<Plugin>,
}

#[derive(Debug)]
pub struct InstallStep {
    pub name: String,
    pub groups: Vec<Group>,
}

#[derive(Debug)]
pub struct Fomod {
    pub name: String,
    pub required: Vec<PathBuf>,
    pub install_steps: Vec<InstallStep>,
}

impl Fomod {
    pub fn parse<C>(config: C) -> Result<Self, Error>
    where
        C: Read,
    {
        let config_tree = Element::parse(config)?;
        let fomod = Fomod {
            // Name
            name: child_text(&config_tree, "moduleName")?.into(),

            // Required install files
            required: {
                let required_install_files = child(&config_tree, "requiredInstallFiles");
                required_install_files
                    .into_iter()
                    .map(|rif| children_attributes(&rif, "file", "source"))
                    .flatten()
                    .map(Into::into)
                    .collect()
            },

            // Install steps
            install_steps: {
                let install_steps = child(&config_tree, "installSteps")?;
                children(&install_steps, "installStep")
                    .map(|step| {
                        Ok(InstallStep {
                            // Name
                            name: step.attributes.get("name").unwrap().into(),

                            // Optional file groups
                            groups: {
                                let groups = child(&step, "optionalFileGroups")?;
                                children(&groups, "group")
                                    .map(|group| {
                                        Ok(Group {
                                            // Name
                                            name: group.attributes.get("name").unwrap().into(),

                                            // Type
                                            ty: toml::from_str(
                                                group.attributes.get("type").unwrap(),
                                            )
                                            .unwrap_or(GroupType::SelectAny),

                                            // Plugins
                                            plugins: {
                                                let plugins = child(&group, "plugins")?;
                                                children(&plugins, "plugin")
                                                    .map(|plugin| {
                                                        Ok(Plugin {
                                                            // Name
                                                            name: plugin
                                                                .attributes
                                                                .get("name")
                                                                .unwrap()
                                                                .into(),

                                                            // Description
                                                            description: child_text(
                                                                plugin,
                                                                "description",
                                                            )
                                                            .unwrap_or_default()
                                                            .into(),

                                                            // Files
                                                            files: {
                                                                child(plugin, "files")
                                                                    .into_iter()
                                                                    .map(|files| {
                                                                        children_attributes(
                                                                            &files, "file",
                                                                            "source",
                                                                        )
                                                                    })
                                                                    .flatten()
                                                                    .map(Into::into)
                                                                    .collect()
                                                            },
                                                        })
                                                    })
                                                    .collect::<Result<Vec<_>, Error>>()?
                                            },
                                        })
                                    })
                                    .collect::<Result<Vec<_>, Error>>()?
                            },
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?
            },
        };
        Ok(fomod)
    }
}

fn child<'a>(elem: &'a Element, name: &str) -> Result<&'a Element, Error> {
    elem.get_child(name)
        .ok_or_else(|| Error::Missing(name.into()))
}

fn children<'a>(elem: &'a Element, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
    elem.children
        .iter()
        .filter_map(|node| node.as_element())
        .filter(move |elem| elem.name == name)
}

fn children_attributes<'a>(
    elem: &'a Element,
    name: &'a str,
    attr: &'a str,
) -> impl Iterator<Item = &'a str> + 'a {
    children(elem, name).filter_map(move |elem| elem.attributes.get(attr).map(|s| s.as_str()))
}

fn child_text<'a>(elem: &'a Element, name: &str) -> Result<Cow<'a, str>, Error> {
    child(elem, name).map(|elem| elem.get_text().unwrap())
}

pub fn pseudo_fomod<P>(top: P) -> crate::Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    let mut path = top.as_ref().to_path_buf();
    // Find main folder
    while fs::read_dir(&path)?.filter_map(Result::ok).count() == 1 {
        path = fs::read_dir(&path)?
            .filter_map(Result::ok)
            .next()
            .unwrap()
            .path();
    }
    let mut install_paths = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(path)?.filter_map(Result::ok).collect();
    entries.sort();
    for entry in entries {
        if entry.file_type()?.is_dir() {
            let lowest = entry.path().iter().last().unwrap().to_owned();
            let starts_with_num = lowest
                .to_string_lossy()
                .chars()
                .next()
                .map_or(false, |c| c.is_digit(10));
            if starts_with_num {
                print!("Would you like to install {:?}? (yes/no) ", lowest);
                stdout().flush()?;
                let input = stdin().lock().lines().next().unwrap()?.to_lowercase();
                if input.starts_with('y') {
                    install_paths.push(entry.path());
                } else if !input.starts_with('n') {
                    println!("Please type yes or no");
                }
            }
        }
    }
    Ok(install_paths)
}
