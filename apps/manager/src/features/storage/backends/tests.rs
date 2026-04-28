use super::local_file::LocalFileControlPlaneBackend;
use nexus_storage::{BackendInstanceId, ControlPlaneBackend, CreateOpts};
use serial_test::serial;
use std::path::PathBuf;
use uuid::Uuid;

fn tmp_storage_root() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[tokio::test]
#[serial]
async fn provision_creates_a_sparse_file_at_requested_size() {
    let root = tmp_storage_root();
    std::env::set_var("MANAGER_STORAGE_ROOT", root.path());

    let backend = LocalFileControlPlaneBackend {
        id: BackendInstanceId(Uuid::new_v4()),
    };
    let h = backend
        .provision(CreateOpts {
            name: "test".into(),
            size_bytes: 16 * 1024 * 1024,
            description: None,
        })
        .await
        .expect("provision");

    let path = PathBuf::from(&h.locator);
    let meta = std::fs::metadata(&path).expect("file exists");
    assert_eq!(meta.len(), 16 * 1024 * 1024);
    assert_eq!(h.size_bytes, 16 * 1024 * 1024);
    assert_eq!(h.backend_kind, nexus_storage::BackendKind::LocalFile);
}

#[tokio::test]
#[serial]
async fn destroy_removes_the_file() {
    let root = tmp_storage_root();
    std::env::set_var("MANAGER_STORAGE_ROOT", root.path());

    let backend = LocalFileControlPlaneBackend {
        id: BackendInstanceId(Uuid::new_v4()),
    };
    let h = backend
        .provision(CreateOpts {
            name: "del".into(),
            size_bytes: 4 * 1024 * 1024,
            description: None,
        })
        .await
        .unwrap();

    let path = PathBuf::from(&h.locator);
    assert!(path.exists());
    backend.destroy(h).await.unwrap();
    assert!(!path.exists());
}

#[tokio::test]
#[serial]
async fn clone_from_image_copies_and_resizes() {
    let root = tmp_storage_root();
    std::env::set_var("MANAGER_STORAGE_ROOT", root.path());

    let src = root.path().join("src.bin");
    let src_size = 4 * 1024 * 1024_u64;
    {
        let f = std::fs::File::create(&src).unwrap();
        f.set_len(src_size).unwrap();
    }

    let backend = LocalFileControlPlaneBackend {
        id: BackendInstanceId(Uuid::new_v4()),
    };
    let h = backend
        .clone_from_image(
            &src,
            CreateOpts {
                name: "cloned".into(),
                size_bytes: src_size,
                description: None,
            },
        )
        .await
        .expect("clone_from_image");

    let cloned_meta = std::fs::metadata(&h.locator).unwrap();
    assert_eq!(cloned_meta.len(), src_size);
}
