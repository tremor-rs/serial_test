use std::panic;

use futures::FutureExt;

use crate::file_lock::make_lock_for_name_and_path;

#[doc(hidden)]
pub fn fs_parallel_core(name: &str, path: Option<&str>, function: fn()) {
    make_lock_for_name_and_path(name, path).start_parallel();
    let res = panic::catch_unwind(|| {
        function();
    });
    make_lock_for_name_and_path(name, path).end_parallel();
    if let Err(err) = res {
        panic::resume_unwind(err);
    }
}

#[doc(hidden)]
pub fn fs_parallel_core_with_return<E>(
    name: &str,
    path: Option<&str>,
    function: fn() -> Result<(), E>,
) -> Result<(), E> {
    make_lock_for_name_and_path(name, path).start_parallel();
    let res = panic::catch_unwind(function);
    make_lock_for_name_and_path(name, path).end_parallel();
    match res {
        Ok(ret) => ret,
        Err(err) => {
            panic::resume_unwind(err);
        }
    }
}

#[doc(hidden)]
pub async fn fs_async_parallel_core_with_return<E>(
    name: &str,
    path: Option<&str>,
    fut: impl std::future::Future<Output = Result<(), E>> + panic::UnwindSafe,
) -> Result<(), E> {
    make_lock_for_name_and_path(name, path).start_parallel();
    let res = fut.catch_unwind().await;
    make_lock_for_name_and_path(name, path).end_parallel();
    match res {
        Ok(ret) => ret,
        Err(err) => {
            panic::resume_unwind(err);
        }
    }
}

#[doc(hidden)]
pub async fn fs_async_parallel_core(
    name: &str,
    path: Option<&str>,
    fut: impl std::future::Future<Output = ()> + panic::UnwindSafe,
) {
    make_lock_for_name_and_path(name, path).start_parallel();
    let res = fut.catch_unwind().await;
    make_lock_for_name_and_path(name, path).end_parallel();
    if let Err(err) = res {
        panic::resume_unwind(err);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        file_lock::{path_for_name, Lock},
        fs_async_parallel_core, fs_async_parallel_core_with_return, fs_parallel_core,
        fs_parallel_core_with_return,
    };
    use std::{io::Error, panic};

    fn unlock_ok(lock_path: &str) {
        let lock = Lock::new(lock_path);
        assert_eq!(lock.parallel_count, 0);
    }

    #[test]
    fn unlock_on_assert_sync_without_return() {
        let lock_path = path_for_name("unlock_on_assert_sync_without_return");
        let _ = panic::catch_unwind(|| {
            fs_parallel_core(
                "unlock_on_assert_sync_without_return",
                Some(&lock_path),
                || {
                    assert!(false);
                },
            )
        });
        unlock_ok(&lock_path);
    }

    #[test]
    fn unlock_on_assert_sync_with_return() {
        let lock_path = path_for_name("unlock_on_assert_sync_with_return");
        let _ = panic::catch_unwind(|| {
            fs_parallel_core_with_return(
                "unlock_on_assert_sync_with_return",
                Some(&lock_path),
                || -> Result<(), Error> {
                    assert!(false);
                    Ok(())
                },
            )
        });
        unlock_ok(&lock_path);
    }

    #[tokio::test]
    async fn unlock_on_assert_async_without_return() {
        let lock_path = path_for_name("unlock_on_assert_async_without_return");
        async fn demo_assert() {
            assert!(false);
        }
        async fn call_serial_test_fn(lock_path: &str) {
            fs_async_parallel_core(
                "unlock_on_assert_async_without_return",
                Some(&lock_path),
                demo_assert(),
            )
            .await
        }

        // as per https://stackoverflow.com/a/66529014/320546
        let _ = panic::catch_unwind(|| {
            let handle = tokio::runtime::Handle::current();
            let _enter_guard = handle.enter();
            futures::executor::block_on(call_serial_test_fn(&lock_path));
        });
        unlock_ok(&lock_path);
    }

    #[tokio::test]
    async fn unlock_on_assert_async_with_return() {
        let lock_path = path_for_name("unlock_on_assert_async_with_return");

        async fn demo_assert() -> Result<(), Error> {
            assert!(false);
            Ok(())
        }

        #[allow(unused_must_use)]
        async fn call_serial_test_fn(lock_path: &str) {
            fs_async_parallel_core_with_return(
                "unlock_on_assert_async_with_return",
                Some(&lock_path),
                demo_assert(),
            )
            .await;
        }

        // as per https://stackoverflow.com/a/66529014/320546
        let _ = panic::catch_unwind(|| {
            let handle = tokio::runtime::Handle::current();
            let _enter_guard = handle.enter();
            futures::executor::block_on(call_serial_test_fn(&lock_path));
        });
        unlock_ok(&lock_path);
    }
}
