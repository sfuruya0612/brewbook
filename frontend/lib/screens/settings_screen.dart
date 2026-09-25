import 'package:flutter/material.dart';

import '../auth/auth_scope.dart';
import '../auth/passkey.dart';
import '../auth/passkey_name.dart';
import '../l10n/app_localizations.dart';
import '../records/values.dart';
import '../settings/settings_services.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';

/// 設定の画面 (FR-3、FR-4、FR-14、FR-15)。
///
/// パスキーの管理、全記録のエクスポート、アカウントと全データの削除、ログアウトを 1 つの
/// 画面にまとめる (設計判断)。パスキーの操作は [AuthScope] の AuthController、エクスポートは
/// [SettingsServices] が担う。
class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key, required this.services});

  /// エクスポートが使う依存 (ADR-0007)。
  final SettingsServices services;

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  /// パスキーの一覧。読み込みが終わるまでは null。
  List<Passkey>? _passkeys;

  /// パスキーの一覧の読み込みの失敗。成功すると消す。
  Object? _passkeysError;

  /// 進行中の操作。二重の実行を防ぐため、操作の間はボタンを無効にする。
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _loadPasskeys();
  }

  /// パスキーの一覧を読み直す (FR-3)。
  Future<void> _loadPasskeys() async {
    final controller = AuthScope.read(context);
    try {
      final passkeys = await controller.passkeys();
      if (!mounted) {
        return;
      }
      setState(() {
        _passkeys = passkeys;
        _passkeysError = null;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _passkeysError = error);
    }
  }

  /// 操作を実行し、失敗を通知で示す。成功の通知は [successMessage] があるときだけ出す。
  Future<void> _run(
    Future<void> Function() action, {
    String? successMessage,
    String Function(Object error, AppLocalizations l10n)? errorMessage,
  }) async {
    final l10n = AppLocalizations.of(context);
    final messenger = ScaffoldMessenger.of(context);
    setState(() => _busy = true);
    try {
      await action();
      if (successMessage != null) {
        messenger.showSnackBar(SnackBar(content: Text(successMessage)));
      }
    } catch (error) {
      messenger.showSnackBar(
        SnackBar(
          content: Text(
            errorMessage?.call(error, l10n) ?? messageForError(error, l10n),
          ),
        ),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// パスキーの名前を入力させる。取り消したときは null を返す (FR-3)。
  Future<String?> _promptPasskeyName({
    required String title,
    required String confirmLabel,
    String? initialName,
  }) {
    return showDialog<String>(
      context: context,
      builder: (context) => _PasskeyNameDialog(
        title: title,
        confirmLabel: confirmLabel,
        initialName: initialName,
      ),
    );
  }

  /// パスキーを追加する (FR-3)。
  Future<void> _addPasskey() async {
    final l10n = AppLocalizations.of(context);
    final name = await _promptPasskeyName(
      title: l10n.addPasskeyTitle,
      confirmLabel: l10n.addButton,
    );
    if (name == null || !mounted) {
      return;
    }
    final controller = AuthScope.read(context);
    await _run(
      () => controller.addPasskey(name: name),
      successMessage: l10n.passkeyAddedMessage,
    );
    if (mounted) {
      await _loadPasskeys();
    }
  }

  /// パスキーの名前を変更する (FR-3)。
  Future<void> _renamePasskey(Passkey passkey) async {
    final l10n = AppLocalizations.of(context);
    final name = await _promptPasskeyName(
      title: l10n.renamePasskeyTitle,
      confirmLabel: l10n.renameButton,
      initialName: passkey.name,
    );
    if (name == null || !mounted) {
      return;
    }
    final controller = AuthScope.read(context);
    await _run(
      () => controller.renamePasskey(id: passkey.id, name: name),
      successMessage: l10n.passkeyRenamedMessage,
    );
    if (mounted) {
      await _loadPasskeys();
    }
  }

  /// パスキーを削除する (FR-3)。最後の 1 つはサーバーが 409 を返す。
  Future<void> _deletePasskey(Passkey passkey) async {
    final l10n = AppLocalizations.of(context);
    final controller = AuthScope.read(context);
    await _run(
      () => controller.deletePasskey(passkey.id),
      successMessage: l10n.passkeyDeletedMessage,
      errorMessage: deletePasskeyErrorMessage,
    );
    if (mounted) {
      await _loadPasskeys();
    }
  }

  /// 全記録をエクスポートし、JSON のファイルとしてダウンロードする (FR-14)。
  Future<void> _export() async {
    final l10n = AppLocalizations.of(context);
    await _run(widget.services.exportAll, successMessage: l10n.exportDoneMessage);
  }

  /// アカウントと全データを削除する (FR-15)。
  ///
  /// 元に戻せないため、確認のダイアログで明示的に確認のボタンを押させ、押したときだけ
  /// 削除の API を呼ぶ。削除に成功するとログインの状態が消え、ルーターがログイン画面へ遷移させる。
  Future<void> _deleteAccount() async {
    final l10n = AppLocalizations.of(context);
    final messenger = ScaffoldMessenger.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l10n.deleteAccountConfirmTitle),
        content: Text(l10n.deleteAccountConfirmMessage),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(l10n.cancelButton),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(l10n.deleteButton),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) {
      return;
    }
    setState(() => _busy = true);
    try {
      await AuthScope.read(context).deleteAccount();
    } catch (error) {
      messenger.showSnackBar(
        SnackBar(content: Text(messageForError(error, l10n))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// ログアウトする (FR-4)。ログアウト後はルーターがログイン画面へ遷移させる。
  Future<void> _logout() async {
    await _run(AuthScope.read(context).logout);
  }

  /// パスキーの行の補足 (登録日時と最終使用日時。FR-3、FR-16)。
  String _passkeySubtitle(BuildContext context, Passkey passkey) {
    final l10n = AppLocalizations.of(context);
    final locale = Localizations.localeOf(context);
    final created = l10n.passkeyCreatedAt(displayTimestamp(passkey.createdAt, locale));
    final lastUsedAt = passkey.lastUsedAt;
    final lastUsed = lastUsedAt == null
        ? l10n.passkeyNotUsedYet
        : l10n.passkeyLastUsedAt(displayTimestamp(lastUsedAt, locale));
    return '$created\n$lastUsed';
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l10n.settingsTitle)),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: <Widget>[
          _SectionTitle(l10n.passkeysTitle),
          Text(l10n.passkeysDescription),
          const SizedBox(height: 8),
          _passkeysSection(l10n),
          const Divider(height: 32),
          _SectionTitle(l10n.exportTitle),
          Text(l10n.exportDescription),
          const SizedBox(height: 8),
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton.icon(
              onPressed: _busy ? null : _export,
              icon: const Icon(Icons.download),
              label: Text(l10n.exportButton),
            ),
          ),
          const Divider(height: 32),
          _SectionTitle(l10n.deleteAccountTitle),
          Text(l10n.deleteAccountDescription),
          const SizedBox(height: 8),
          Align(
            alignment: Alignment.centerLeft,
            child: FilledButton(
              onPressed: _busy ? null : _deleteAccount,
              style: FilledButton.styleFrom(
                backgroundColor: Theme.of(context).colorScheme.error,
                foregroundColor: Theme.of(context).colorScheme.onError,
              ),
              child: Text(l10n.deleteAccountButton),
            ),
          ),
          const Divider(height: 32),
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton(
              onPressed: _busy ? null : _logout,
              child: Text(l10n.logoutButton),
            ),
          ),
        ],
      ),
    );
  }

  /// パスキーの節 (一覧、追加、名前の変更、削除。FR-3)。
  Widget _passkeysSection(AppLocalizations l10n) {
    final error = _passkeysError;
    if (error != null) {
      return ErrorBanner(
        message: messageForError(error, l10n),
        onRetry: _loadPasskeys,
      );
    }
    final passkeys = _passkeys;
    if (passkeys == null) {
      return const Center(child: CircularProgressIndicator());
    }
    // パスキーが 1 つしかないときは削除できない (設計判断)。サーバーも 409 を返す。
    final canDelete = passkeys.length > 1;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        for (final passkey in passkeys)
          ListTile(
            contentPadding: EdgeInsets.zero,
            title: Text(passkey.name),
            subtitle: Text(_passkeySubtitle(context, passkey)),
            trailing: Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                IconButton(
                  tooltip: l10n.renameButton,
                  icon: const Icon(Icons.edit_outlined),
                  onPressed: _busy ? null : () => _renamePasskey(passkey),
                ),
                IconButton(
                  tooltip: l10n.deleteButton,
                  icon: const Icon(Icons.delete_outline),
                  onPressed: _busy || !canDelete ? null : () => _deletePasskey(passkey),
                ),
              ],
            ),
          ),
        const SizedBox(height: 8),
        Align(
          alignment: Alignment.centerLeft,
          child: OutlinedButton.icon(
            onPressed: _busy ? null : _addPasskey,
            icon: const Icon(Icons.add),
            label: Text(l10n.addPasskeyButton),
          ),
        ),
      ],
    );
  }
}

/// 設定の画面の節の見出し。
class _SectionTitle extends StatelessWidget {
  const _SectionTitle(this.title);

  /// 見出しの文言 (ARB から取る。FR-16)。
  final String title;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Text(title, style: Theme.of(context).textTheme.titleMedium),
    );
  }
}

/// パスキーの名前を入力させるダイアログ (FR-3)。
///
/// 確定した名前 (前後の空白を除く) を返す。取り消したときは null を返す。
class _PasskeyNameDialog extends StatefulWidget {
  const _PasskeyNameDialog({
    required this.title,
    required this.confirmLabel,
    this.initialName,
  });

  /// ダイアログの見出し。
  final String title;

  /// 確定のボタンの文言。
  final String confirmLabel;

  /// 変更のときの現在の名前。追加のときは null。
  final String? initialName;

  @override
  State<_PasskeyNameDialog> createState() => _PasskeyNameDialogState();
}

class _PasskeyNameDialogState extends State<_PasskeyNameDialog> {
  late final TextEditingController _name = TextEditingController(
    text: widget.initialName,
  );

  /// 名前の検証の失敗の文言。
  String? _error;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  /// 名前を検証し、正しければ入力の名前を返して閉じる。
  void _confirm() {
    final l10n = AppLocalizations.of(context);
    final error = validatePasskeyName(_name.text, l10n);
    if (error != null) {
      setState(() => _error = error);
      return;
    }
    Navigator.of(context).pop(passkeyNameForRequest(_name.text));
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return AlertDialog(
      title: Text(widget.title),
      content: TextField(
        controller: _name,
        autofocus: true,
        decoration: InputDecoration(
          labelText: l10n.passkeyNameLabel,
          hintText: l10n.passkeyNameHint,
          helperText: l10n.passkeyNameHelper,
          errorText: _error,
        ),
        onSubmitted: (_) => _confirm(),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancelButton),
        ),
        FilledButton(
          onPressed: _confirm,
          child: Text(widget.confirmLabel),
        ),
      ],
    );
  }
}
