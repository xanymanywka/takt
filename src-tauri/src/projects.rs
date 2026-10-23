use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

fn projects_path() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".orchestra/projects.json"))
        .ok_or_else(|| "Не удалось определить домашнюю папку".to_owned())
}

#[tauri::command]
pub fn list_projects() -> Result<Vec<Project>, String> {
    let path = projects_path()?;
    if path.exists() {
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Не удалось прочитать {}: {error}", path.display()))?;
        return serde_json::from_str(&content)
            .map_err(|error| format!("Некорректный projects.json: {error}"));
    }

    let root = dirs::home_dir()
        .ok_or_else(|| "Не удалось определить домашнюю папку".to_owned())?
        .join("projects");
    let mut projects = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let candidate = entry.path();
            if candidate.is_dir() && candidate.join(".git").exists() {
                projects.push(Project {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path: candidate.to_string_lossy().into_owned(),
                    group: None,
                    color: None,
                });
            }
        }
    }
    projects.sort_by_key(|project| project.name.to_lowercase());
    Ok(projects)
}

#[tauri::command]
pub fn save_projects(projects: Vec<Project>) -> Result<(), String> {
    let path = projects_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Не удалось создать {}: {error}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&projects)
        .map_err(|error| format!("Не удалось сериализовать проекты: {error}"))?;
    fs::write(&path, format!("{json}\n"))
        .map_err(|error| format!("Не удалось записать {}: {error}", path.display()))
}
