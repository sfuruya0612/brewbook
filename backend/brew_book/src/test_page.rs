//! 仮想認証器を使う結合テストが使うページ (0005 のテスト、ADR-0004)。
//!
//! PRD の API ではないため経路の台帳 (0001) に載せない。`TEST_PAGE` の var が `true` のときだけ
//! 応答し、それ以外は台帳に無い経路と同じ 404 になる。結合テストは
//! `wrangler dev --var TEST_PAGE:true` で有効にする (本番の vars には無い)。
//!
//! `/api/*` 以外の経路は Static Assets が返し、Static Assets は静的ファイルの無いパスに
//! `index.html` を返す (ADR-0005)。Worker が処理するページは `d1_check` と同じく `/api/` の下に置く。
//!
//! ページは、同じオリジンの API を呼び、`navigator.credentials` でパスキーを作る・使うための
//! 最小限の関数を持つ。仮想認証器はテストが ChromeDriver の WebAuthn の拡張コマンドで付ける。

use worker::{Env, Method, Request, Response, Result};

/// このページのパス。`/api/*` は Worker が先に処理する (`run_worker_first`、ADR-0005)。
pub const PATH: &str = "/api/__test_page";
/// このページの経路名。ログの `route` に使う。
pub const ROUTE_NAME: &str = "test_page";
/// このページを有効にする vars の名前。
pub const VAR_NAME: &str = "TEST_PAGE";
/// このページが有効な値。
const VAR_ENABLED: &str = "true";

/// 有効ならページを返す。無効な経路は None を返し、呼び出し側が 404 にする。
pub async fn run(req: &Request, env: &Env) -> Option<Result<Response>> {
    if req.path() != PATH || req.method() != Method::Get || !is_enabled(env) {
        return None;
    }
    Some(Response::from_html(PAGE))
}

/// `TEST_PAGE` の var が `true` のときだけ有効にする。
fn is_enabled(env: &Env) -> bool {
    matches!(env.var(VAR_NAME), Ok(var) if var.to_string() == VAR_ENABLED)
}

/// テストページの HTML。関数は `window.brewBookTest` から呼ぶ。
const PAGE: &str = r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="utf-8">
<title>brewbook test page</title>
</head>
<body>
<p>brewbook test page</p>
<script>
"use strict";

// base64url の文字列をバイト列にする (W3C WebAuthn Level 3 の 3)。
function fromBase64Url(text) {
  const normalized = text.replace(/-/g, "+").replace(/_/g, "/");
  const padded = normalized + "=".repeat((4 - (normalized.length % 4)) % 4);
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

// 同じオリジンの API を呼び、状態と本文を返す。
// fetch の既定の資格情報は同一オリジンなので、セッションの Cookie は自動で送られる。
async function call(path, options = {}) {
  const init = { method: options.method || "GET" };
  if (options.body !== undefined) {
    init.headers = { "content-type": "application/json" };
    init.body = JSON.stringify(options.body);
  }
  const response = await fetch(path, init);
  const text = await response.text();
  let body = null;
  if (text) {
    try {
      body = JSON.parse(text);
    } catch (error) {
      body = text;
    }
  }
  return { status: response.status, body };
}

// サーバーの作成のオプションを navigator.credentials.create に渡せる形にする。
// サーバーは user.displayName を設定しないため (ADR-0004)、必須のメンバーをここで埋める。
function creationOptions(options) {
  return {
    publicKey: {
      challenge: fromBase64Url(options.challenge),
      rp: options.rp,
      user: {
        id: fromBase64Url(options.user.id),
        name: options.user.name,
        displayName: options.user.name
      },
      pubKeyCredParams: options.pubKeyCredParams,
      attestation: options.attestation,
      authenticatorSelection: options.authenticatorSelection,
      timeout: options.timeout
    }
  };
}

// サーバーの要求のオプションを navigator.credentials.get に渡せる形にする。
function requestOptions(options) {
  return {
    publicKey: {
      challenge: fromBase64Url(options.challenge),
      rpId: options.rpId,
      userVerification: options.userVerification,
      allowCredentials: options.allowCredentials,
      timeout: options.timeout
    }
  };
}

// パスキーを作り、JSON の表記で返す。
async function createCredential(options) {
  const credential = await navigator.credentials.create(creationOptions(options));
  return JSON.parse(JSON.stringify(credential));
}

// パスキーで署名し、JSON の表記で返す。
async function getCredential(options) {
  const credential = await navigator.credentials.get(requestOptions(options));
  return JSON.parse(JSON.stringify(credential));
}

// 登録用トークンでパスキーを登録する (FR-1)。
async function registerWithToken(token, name) {
  const begin = await call("/api/auth/register/begin", { method: "POST", body: { token } });
  if (begin.status !== 200) {
    return { status: begin.status, body: begin.body };
  }
  const credential = await createCredential(begin.body);
  const complete = await call("/api/auth/register/complete", {
    method: "POST",
    body: { token, name, credential }
  });
  return { status: complete.status, body: complete.body, credential };
}

// 登録の検証だけを呼ぶ (使用済みのトークンの検査に使う)。
function completeRegistration(token, name, credential) {
  return call("/api/auth/register/complete", {
    method: "POST",
    body: { token, name, credential }
  });
}

// ログインする (FR-2)。
async function login() {
  const begin = await call("/api/auth/login/begin", { method: "POST" });
  if (begin.status !== 200) {
    return { status: begin.status, body: begin.body };
  }
  const credential = await getCredential(begin.body);
  const complete = await call("/api/auth/login/complete", {
    method: "POST",
    body: { credential }
  });
  return { status: complete.status, body: complete.body, credential };
}

// ログインのチャレンジだけを発行する (チャレンジの状態の検査に使う)。
function beginLogin() {
  return call("/api/auth/login/begin", { method: "POST" });
}

// パスキーで署名し、検証には送らない (検証の失敗の検査に使う)。
async function getAssertion() {
  const begin = await call("/api/auth/login/begin", { method: "POST" });
  if (begin.status !== 200) {
    return { status: begin.status, body: begin.body };
  }
  const credential = await getCredential(begin.body);
  return { status: begin.status, body: begin.body, credential };
}

// ログインの検証だけを呼ぶ (使用済みのチャレンジの検査に使う)。
function completeLogin(credential) {
  return call("/api/auth/login/complete", { method: "POST", body: { credential } });
}

// 署名を付けずにログインの検証を呼ぶ (チャレンジの状態の検査に使う)。
function completeLoginWithChallenge(challenge) {
  return call("/api/auth/login/complete", {
    method: "POST",
    body: { credential: { id: "unknown-credential", response: {
      clientDataJSON: clientData("webauthn.get", challenge),
      authenticatorData: "",
      signature: ""
    } } }
  });
}

// 署名だけを壊したクレデンシャルでログインの検証を呼ぶ (検証の失敗の検査に使う)。
function completeLoginWithBadSignature(credential) {
  credential.response.signature = "AAAA";
  return call("/api/auth/login/complete", { method: "POST", body: { credential } });
}

// パスキーを追加する (FR-3)。
async function addPasskey(name) {
  const begin = await call("/api/passkeys/begin", { method: "POST" });
  if (begin.status !== 200) {
    return { status: begin.status, body: begin.body };
  }
  const credential = await createCredential(begin.body);
  const complete = await call("/api/passkeys/complete", {
    method: "POST",
    body: { name, credential }
  });
  return { status: complete.status, body: complete.body, credential };
}

// 署名を付けないクライアントデータを作る (チャレンジの期限切れと検証の失敗の検査に使う)。
function clientData(type, challenge) {
  const json = JSON.stringify({ type, challenge, origin: location.origin });
  return btoa(json).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

window.brewBookTest = {
  call,
  registerWithToken,
  completeRegistration,
  login,
  beginLogin,
  getAssertion,
  completeLogin,
  completeLoginWithChallenge,
  completeLoginWithBadSignature,
  addPasskey,
  clientData,
  listPasskeys: () => call("/api/passkeys"),
  renamePasskey: (id, name) => call("/api/passkeys/" + encodeURIComponent(id), {
    method: "PATCH",
    body: { name }
  }),
  deletePasskey: (id) => call("/api/passkeys/" + encodeURIComponent(id), { method: "DELETE" }),
  logout: () => call("/api/auth/logout", { method: "POST" })
};
</script>
</body>
</html>
"#;
