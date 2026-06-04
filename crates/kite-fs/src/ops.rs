use std::path::Path;
use tokio::fs;
use tokio::sync::mpsc::UnboundedSender;

pub async fn copy_entries(
    sources: &[std::path::PathBuf],
    dest: &Path,
    progress_tx: Option<UnboundedSender<(usize, usize)>>,
) -> std::io::Result<usize> {
    let total = sources.len();
    let mut completed = 0;

    for source in sources {
        let target = dest.join(source.file_name().unwrap_or_default());
        copy_recursive(source, &target).await?;
        completed += 1;
        if let Some(ref tx) = progress_tx {
            let _ = tx.send((completed, total));
        }
    }

    Ok(completed)
}

pub async fn move_entries(
    sources: &[std::path::PathBuf],
    dest: &Path,
    progress_tx: Option<UnboundedSender<(usize, usize)>>,
) -> std::io::Result<usize> {
    let total = sources.len();
    let mut completed = 0;

    for source in sources {
        let target = dest.join(source.file_name().unwrap_or_default());
        // Try rename first (same filesystem), fall back to copy+delete
        if fs::rename(source, &target).await.is_err() {
            copy_recursive(source, &target).await?;
            delete_recursive(source).await?;
        }
        completed += 1;
        if let Some(ref tx) = progress_tx {
            let _ = tx.send((completed, total));
        }
    }

    Ok(completed)
}

pub async fn delete_entries(
    targets: &[std::path::PathBuf],
    progress_tx: Option<UnboundedSender<(usize, usize)>>,
) -> std::io::Result<usize> {
    let total = targets.len();
    let mut completed = 0;

    for target in targets {
        delete_recursive(target).await?;
        completed += 1;
        if let Some(ref tx) = progress_tx {
            let _ = tx.send((completed, total));
        }
    }

    Ok(completed)
}

pub async fn rename_entry(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::rename(from, to).await
}

pub async fn create_file(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::File::create(path).await?;
    Ok(())
}

pub async fn create_directory(path: &Path) -> std::io::Result<()> {
    fs::create_dir_all(path).await
}

fn copy_recursive<'a>(
    src: &'a Path,
    dst: &'a Path,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'a>> {
    Box::pin(async move {
        let metadata = fs::metadata(src).await?;

        if metadata.is_dir() {
            fs::create_dir_all(dst).await?;
            let mut dir = fs::read_dir(src).await?;
            while let Some(entry) = dir.next_entry().await? {
                let child_src = entry.path();
                let child_dst = dst.join(entry.file_name());
                copy_recursive(&child_src, &child_dst).await?;
            }
        } else {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::copy(src, dst).await?;
        }

        Ok(())
    })
}

async fn delete_recursive(path: &Path) -> std::io::Result<()> {
    let metadata = fs::symlink_metadata(path).await?;

    if metadata.is_dir() {
        fs::remove_dir_all(path).await
    } else {
        fs::remove_file(path).await
    }
}
