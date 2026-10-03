use crate::{fs_safety, platform};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: String,
    pub manifest_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsOverview {
    pub directory: String,
    pub skills: Vec<Skill>,
    pub skipped: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pet {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub sprite_version_number: u8,
    pub sprite_file: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetsOverview {
    pub directory: String,
    pub pets: Vec<Pet>,
    pub skipped: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PetManifest {
    id: String,
    display_name: String,
    description: String,
    sprite_version_number: u8,
    spritesheet_path: String,
}

const SKILL_MANIFEST: &str = "SKILL.md";
const PET_MANIFEST: &str = "pet.json";

fn display(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
fn skills_root() -> PathBuf {
    platform::codex_home().join("skills")
}
fn pets_root() -> PathBuf {
    platform::codex_home().join("pets")
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn regular_file(path: &Path) -> Result<fs::Metadata, String> {
    fs_safety::check(path)?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| format!("Required file is unavailable: {}", display(path)))?;
    if fs_safety::linked(&metadata) || !metadata.is_file() {
        return Err(
            "Required file must be a regular file, not a symbolic link or directory.".into(),
        );
    }
    Ok(metadata)
}

fn existing_directory(path: &Path) -> Result<bool, String> {
    fs_safety::check(path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if !fs_safety::linked(&metadata) && metadata.is_dir() => Ok(true),
        Ok(_) => Err("Expected a regular directory, not a symbolic link or file.".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("Cannot inspect directory: {error}")),
    }
}

fn create_directory(path: &Path) -> Result<(), String> {
    if !existing_directory(path)? {
        fs::create_dir_all(path).map_err(|error| format!("Cannot create directory: {error}"))?;
    }
    if !existing_directory(path)? {
        return Err("Directory could not be prepared safely.".into());
    }
    Ok(())
}

fn direct_child(root: &Path, id: &str) -> Result<PathBuf, String> {
    if !valid_id(id) {
        return Err("The selected item has an unsafe name.".into());
    }
    let path = root.join(id);
    fs_safety::check(&path)?;
    Ok(path)
}

fn skill_at(path: &Path) -> Result<Skill, String> {
    if !existing_directory(path)? {
        return Err("Skill directory is unavailable.".into());
    }
    let id = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| valid_id(name))
        .ok_or("Skill directory name is unsupported.")?
        .to_string();
    let manifest = regular_file(&path.join(SKILL_MANIFEST))?;
    Ok(Skill {
        id,
        manifest_bytes: manifest.len(),
    })
}

fn valid_sprite_path(value: &str) -> bool {
    let path = Path::new(value);
    path.components().count() == 1
        && path.file_name().and_then(|name| name.to_str()) == Some(value)
        && matches!(
            path.extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("png" | "webp")
        )
}

fn sprite_has_supported_header(path: &Path) -> Result<(), String> {
    let mut bytes = [0_u8; 12];
    let count = fs::File::open(path)
        .map_err(|_| "Cannot read the pet sprite file.")?
        .read(&mut bytes)
        .map_err(|_| "Cannot read the pet sprite file.")?;
    let png = count >= 8 && bytes[..8] == *b"\x89PNG\r\n\x1a\n";
    let webp = count >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP";
    if png || webp {
        Ok(())
    } else {
        Err("Pet sprite must be a PNG or WebP image.".into())
    }
}

fn pet_at(path: &Path) -> Result<Pet, String> {
    if !existing_directory(path)? {
        return Err("Pet directory is unavailable.".into());
    }
    let directory_id = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| valid_id(name))
        .ok_or("Pet directory name is unsupported.")?;
    let manifest_file = regular_file(&path.join(PET_MANIFEST))?;
    if manifest_file.len() > 64 * 1024 {
        return Err("pet.json is too large to be a supported pet manifest.".into());
    }
    let manifest: PetManifest = serde_json::from_slice(
        &fs::read(path.join(PET_MANIFEST)).map_err(|_| "Cannot read pet.json.")?,
    )
    .map_err(|_| "pet.json does not match the supported pet v2 manifest.")?;
    if manifest.id != directory_id
        || !valid_id(&manifest.id)
        || manifest.display_name.trim().is_empty()
        || manifest.display_name.len() > 80
        || manifest.description.trim().is_empty()
        || manifest.description.len() > 500
        || manifest.sprite_version_number != 2
        || !valid_sprite_path(&manifest.spritesheet_path)
    {
        return Err("pet.json does not match the supported pet v2 contract.".into());
    }
    let sprite = path.join(&manifest.spritesheet_path);
    regular_file(&sprite)?;
    sprite_has_supported_header(&sprite)?;
    Ok(Pet {
        id: manifest.id,
        display_name: manifest.display_name,
        description: manifest.description,
        sprite_version_number: manifest.sprite_version_number,
        sprite_file: manifest.spritesheet_path,
    })
}

fn list_directories<T>(
    root: &Path,
    inspect: impl Fn(&Path) -> Result<T, String>,
) -> Result<(Vec<T>, usize), String> {
    if !existing_directory(root)? {
        return Ok((Vec::new(), 0));
    }
    let mut items = Vec::new();
    let mut skipped = 0;
    for entry in fs::read_dir(root).map_err(|error| format!("Cannot list directory: {error}"))? {
        let Ok(entry) = entry else {
            skipped += 1;
            continue;
        };
        let path = entry.path();
        match fs::symlink_metadata(&path) {
            Ok(metadata) if !fs_safety::linked(&metadata) && metadata.is_dir() => {
                match inspect(&path) {
                    Ok(item) => items.push(item),
                    Err(_) => skipped += 1,
                }
            }
            _ => skipped += 1,
        }
    }
    Ok((items, skipped))
}

pub fn skills() -> Result<SkillsOverview, String> {
    let root = skills_root();
    let (mut skills, skipped) = list_directories(&root, skill_at)?;
    skills.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(SkillsOverview {
        directory: display(&root),
        skills,
        skipped,
    })
}

pub fn pets() -> Result<PetsOverview, String> {
    let root = pets_root();
    let (mut pets, skipped) = list_directories(&root, pet_at)?;
    pets.sort_by(|left, right| left.display_name.cmp(&right.display_name));
    Ok(PetsOverview {
        directory: display(&root),
        pets,
        skipped,
    })
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    if !existing_directory(source)? {
        return Err("Source directory is unavailable.".into());
    }
    for entry in
        fs::read_dir(source).map_err(|error| format!("Cannot read source directory: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Cannot read source entry: {error}"))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        fs_safety::check(&source_path)?;
        let metadata =
            fs::symlink_metadata(&source_path).map_err(|_| "Source entry is unavailable.")?;
        if fs_safety::linked(&metadata) {
            return Err("Source folders cannot contain symbolic links or junctions.".into());
        }
        if metadata.is_dir() {
            fs::create_dir(&destination_path)
                .map_err(|error| format!("Cannot prepare destination: {error}"))?;
            copy_tree(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path)
                .map_err(|error| format!("Cannot copy source file: {error}"))?;
        } else {
            return Err("Source folders can contain only regular files and directories.".into());
        }
    }
    Ok(())
}

fn temporary_directory(parent: &Path, id: &str) -> Result<PathBuf, String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "System time is unavailable.")?
        .as_nanos();
    let temporary = parent.join(format!(".{id}.dev-companion-{nonce}"));
    fs::create_dir(&temporary).map_err(|_| "Cannot prepare a temporary destination directory.")?;
    Ok(temporary)
}

fn copy_skill_to(source: &Path, parent: &Path) -> Result<ActionResult, String> {
    let skill = skill_at(source)?;
    create_directory(parent)?;
    let destination = direct_child(parent, &skill.id)?;
    if destination.exists() {
        return Err(
            "A skill with this name already exists at the destination; nothing was overwritten."
                .into(),
        );
    }
    let temporary = temporary_directory(parent, &skill.id)?;
    let result = copy_tree(source, &temporary).and_then(|_| {
        fs::rename(&temporary, &destination)
            .map_err(|error| format!("Cannot publish skill: {error}"))
    });
    if result.is_err() {
        let _ = fs::remove_dir_all(&temporary);
    }
    result?;
    Ok(ActionResult {
        path: display(&destination),
    })
}

pub fn import_skill(source: &Path) -> Result<ActionResult, String> {
    copy_skill_to(source, &skills_root())
}

pub fn export_skill(id: &str, destination: &Path) -> Result<ActionResult, String> {
    let source = direct_child(&skills_root(), id)?;
    copy_skill_to(&source, destination)
}

fn copy_pet_to(source: &Path) -> Result<ActionResult, String> {
    let pet = pet_at(source)?;
    let root = pets_root();
    create_directory(&root)?;
    let destination = direct_child(&root, &pet.id)?;
    if destination.exists() {
        return Err("A pet with this ID already exists; nothing was overwritten.".into());
    }
    let temporary = temporary_directory(&root, &pet.id)?;
    let result = (|| {
        fs::copy(source.join(PET_MANIFEST), temporary.join(PET_MANIFEST))
            .map_err(|error| format!("Cannot copy pet manifest: {error}"))?;
        fs::copy(
            source.join(&pet.sprite_file),
            temporary.join(&pet.sprite_file),
        )
        .map_err(|error| format!("Cannot copy pet sprite: {error}"))?;
        fs::rename(&temporary, &destination).map_err(|error| format!("Cannot publish pet: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&temporary);
    }
    result?;
    Ok(ActionResult {
        path: display(&destination),
    })
}

pub fn install_pet(source: &Path) -> Result<ActionResult, String> {
    copy_pet_to(source)
}

fn ensure_regular_tree(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "Selected pet is unavailable.")?;
    if fs_safety::linked(&metadata) {
        return Err(
            "Pet contains a symbolic link or junction and cannot be removed safely.".into(),
        );
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(|_| "Cannot inspect pet before removal.")? {
            ensure_regular_tree(
                &entry
                    .map_err(|_| "Cannot inspect pet before removal.")?
                    .path(),
            )?;
        }
    } else if !metadata.is_file() {
        return Err("Pet contains an unsupported filesystem entry.".into());
    }
    Ok(())
}

fn remove_pet_at(root: &Path, id: &str, confirmation: &str) -> Result<ActionResult, String> {
    if confirmation != "REMOVE" {
        return Err("Type REMOVE to remove this pet.".into());
    }
    let path = direct_child(root, id)?;
    if !existing_directory(&path)? {
        return Err("Selected pet is unavailable.".into());
    }
    ensure_regular_tree(&path)?;
    fs::remove_dir_all(&path).map_err(|error| format!("Cannot remove pet: {error}"))?;
    Ok(ActionResult {
        path: display(&path),
    })
}

pub fn remove_pet(id: &str, confirmation: &str) -> Result<ActionResult, String> {
    remove_pet_at(&pets_root(), id, confirmation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dev-companion-content-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
    fn write(path: &Path, contents: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    fn pet_manifest(id: &str) -> String {
        format!(
            r#"{{"id":"{id}","displayName":"{id}","description":"A test pet.","spriteVersionNumber":2,"spritesheetPath":"sprite.webp"}}"#
        )
    }
    const WEBP: &[u8] = b"RIFF\0\0\0\0WEBPVP8 ";

    #[test]
    fn lists_only_direct_skills_with_manifests() {
        let root = root("skills");
        write(&root.join("valid/SKILL.md"), b"---\nname: Valid\n---\n");
        write(&root.join("missing/readme.md"), b"not a skill");
        let (skills, skipped) = list_directories(&root, skill_at).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].id, "valid");
        assert_eq!(skipped, 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn validates_pet_v2_manifest_and_sprite() {
        let root = root("pets");
        let pet = root.join("boba");
        write(&pet.join(PET_MANIFEST), pet_manifest("boba").as_bytes());
        write(&pet.join("sprite.webp"), WEBP);
        assert_eq!(pet_at(&pet).unwrap().display_name, "boba");
        write(&pet.join(PET_MANIFEST), pet_manifest("other").as_bytes());
        assert!(pet_at(&pet).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn copies_skills_to_a_new_destination_only() {
        let root = root("copy");
        let source = root.join("source/example");
        let destination = root.join("destination");
        write(&source.join("SKILL.md"), b"---\nname: Example\n---\n");
        write(&source.join("assets/example.txt"), b"ok");
        let result = copy_skill_to(&source, &destination).unwrap();
        assert!(Path::new(&result.path).join("assets/example.txt").is_file());
        assert!(copy_skill_to(&source, &destination).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn removes_only_a_confirmed_direct_pet() {
        let root = root("remove");
        let pet = root.join("boba");
        write(&pet.join(PET_MANIFEST), pet_manifest("boba").as_bytes());
        write(&pet.join("sprite.webp"), WEBP);
        assert!(remove_pet_at(&root, "boba", "no").is_err());
        assert!(pet.is_dir());
        remove_pet_at(&root, "boba", "REMOVE").unwrap();
        assert!(!pet.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
