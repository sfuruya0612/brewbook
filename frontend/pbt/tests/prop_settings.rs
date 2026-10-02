//! 設定の画面の PBT (0043)。
//!
//! アカウント削除の確認の状態機械の性質を、任意の操作の列で確かめる (FR-15、ADR-0013)。

#![cfg(not(target_arch = "wasm32"))]

use brew_book_frontend::screens::settings::AccountDelete;
use proptest::prelude::*;

/// 確認ダイアログへの操作。
#[derive(Clone, Copy, Debug)]
enum Op {
    /// 入口のボタンで確認を出す。
    Request,
    /// 取り消す。
    Cancel,
    /// 確認のボタンを押す。
    Confirm,
}

/// 任意の操作。
fn op() -> impl Strategy<Value = Op> {
    prop_oneof![Just(Op::Request), Just(Op::Cancel), Just(Op::Confirm)]
}

proptest! {
    /// 削除の API を呼ぶのは、確認を出した後に確認のボタンを押したときだけ (FR-15)。
    #[test]
    fn the_account_delete_calls_the_api_only_after_the_confirmation(
        ops in prop::collection::vec(op(), 0..40),
    ) {
        let mut dialog = AccountDelete::new();
        let mut open = false;
        let mut requests = 0_usize;
        let mut calls = 0_usize;
        for op in ops {
            match op {
                Op::Request => {
                    dialog.request();
                    open = true;
                    requests += 1;
                }
                Op::Cancel => {
                    dialog.cancel();
                    open = false;
                }
                Op::Confirm => {
                    let call = dialog.confirm();
                    prop_assert_eq!(call, open, "the API call must match the open state");
                    if call {
                        calls += 1;
                    }
                    open = false;
                }
            }
            prop_assert_eq!(dialog.is_open(), open);
        }
        // 呼び出しの回数は、入口のボタンを押した回数を超えない。
        prop_assert!(calls <= requests);
    }
}
