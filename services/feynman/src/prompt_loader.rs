use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn load_prompts(dir_path: &Path) -> Result<HashMap<String, String>> {
    let mut prompts = HashMap::new();

    for entry in fs::read_dir(dir_path)
        .with_context(|| format!("Failed to read prompts directory: {}", dir_path.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            let prompt_key = path
                .file_stem()
                .and_then(|s| s.to_str())
                .context("Could not get file stem for prompt file")?
                .to_string();

            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read prompt file: {}", path.display()))?;

            prompts.insert(prompt_key, content);
        }
    }

    Ok(prompts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_prompts_successfully() -> Result<()> {
        // 1. Arrange: Create a temporary directory and some mock prompt files.
        let dir = tempdir()?;
        let dir_path = dir.path();

        // Create a valid prompt file.
        // The `{{...}}` in `writeln!` escapes the braces, so `{placeholder1}` is written to the file.
        let mut file1 = File::create(dir_path.join("prompt1.md"))?;
        writeln!(file1, "This is prompt 1. Placeholder: {{placeholder1}}")?;

        // Create another valid prompt file.
        let mut file2 = File::create(dir_path.join("prompt2.md"))?;
        writeln!(file2, "This is prompt 2.")?;

        // Create a file that should be ignored (not .md).
        let mut ignored_file = File::create(dir_path.join("config.txt"))?;
        writeln!(ignored_file, "some config")?;

        // Create a subdirectory that should be ignored.
        std::fs::create_dir(dir_path.join("subdir"))?;

        // 2. Act: Call the function to load prompts.
        let prompts = load_prompts(dir_path)?;

        // 3. Assert: Check if the prompts were loaded correctly.
        assert_eq!(prompts.len(), 2, "Should only load .md files");

        // The assertion should expect single braces, which is what is actually in the file.
        assert_eq!(
            prompts.get("prompt1").unwrap(),
            "This is prompt 1. Placeholder: {placeholder1}\n"
        );
        assert_eq!(prompts.get("prompt2").unwrap(), "This is prompt 2.\n");

        assert!(
            prompts.get("config").is_none(),
            "Should not load .txt files"
        );
        assert!(
            prompts.get("config.txt").is_none(),
            "Key should be the file stem, not the full name"
        );

        Ok(())
    }

    #[test]
    fn test_load_prompts_from_nonexistent_dir() {
        // Arrange: Path to a directory that does not exist.
        let dir_path = Path::new("nonexistent_dir_for_testing_prompts");

        // Act: Call the function.
        let result = load_prompts(dir_path);

        // Assert: The function should return an error.
        assert!(result.is_err());
    }

    #[test]
    fn test_load_prompts_from_empty_dir() -> Result<()> {
        // Arrange: Create an empty temporary directory.
        let dir = tempdir()?;
        let dir_path = dir.path();

        // Act: Call the function.
        let prompts = load_prompts(dir_path)?;

        // Assert: The resulting map should be empty.
        assert!(prompts.is_empty());

        Ok(())
    }
}
