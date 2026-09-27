import 'package:flutter/material.dart';

import '../auth/auth_scope.dart';
import '../auth/passkey.dart';
import '../auth/passkey_name.dart';
import '../l10n/app_localizations.dart';
import '../records/values.dart';
import '../settings/settings_services.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/app_field.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';
import '../widgets/wide_layout.dart';

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
    final brewbook = BrewbookTheme.of(context);
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
            style: FilledButton.styleFrom(
              backgroundColor: brewbook.palette.signal,
              foregroundColor: brewbook.palette.onSignal,
            ),
            child: Text(l10n.deleteConfirmButton),
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
    return '$created / $lastUsed';
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final content = Scaffold(
      appBar: AppBar(title: Text(l10n.settingsTitle)),
      body: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            _Section(
              title: l10n.passkeysTitle,
              description: l10n.passkeysDescription,
              child: _passkeysSection(l10n),
            ),
            _Section(
              title: l10n.exportTitle,
              description: l10n.exportDescription,
              child: Align(
                alignment: Alignment.centerLeft,
                child: _secondaryButton(
                  context,
                  onPressed: _busy ? null : _export,
                  icon: Icons.download_outlined,
                  label: l10n.exportButton,
                ),
              ),
            ),
            _Section(
              title: l10n.deleteAccountTitle,
              description: l10n.deleteAccountDescription,
              child: Align(
                alignment: Alignment.centerLeft,
                child: _dangerButton(
                  context,
                  onPressed: _busy ? null : _deleteAccount,
                  label: l10n.deleteAccountButton,
                ),
              ),
            ),
            _Section(
              child: Align(
                alignment: Alignment.centerLeft,
                child: TextButton.icon(
                  onPressed: _busy ? null : _logout,
                  icon: const Icon(Icons.logout_outlined, size: 20),
                  label: Text(l10n.logoutButton),
                ),
              ),
            ),
          ],
        ),
      ),
    );
    return LayoutBuilder(
      builder: (context, constraints) {
        final wide = constraints.maxWidth >= wideLayoutBreakpoint;
        if (!wide) {
          return content;
        }
        return Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            const BrewbookNavigationRail(selectedIndex: 5),
            Expanded(child: content),
          ],
        );
      },
    );
  }

  /// 次点のボタン (枠だけの 36 px)。
  Widget _secondaryButton(
    BuildContext context, {
    required VoidCallback? onPressed,
    required IconData icon,
    required String label,
  }) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return OutlinedButton.icon(
      onPressed: onPressed,
      icon: Icon(icon, size: 20),
      label: Text(label),
      style: OutlinedButton.styleFrom(
        minimumSize: const Size(0, 36),
        padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x4),
        textStyle: AppTextStyle.label(color: brewbook.palette.ink),
        iconColor: brewbook.palette.ink,
      ),
    );
  }

  /// 取り消せない操作の入口 (signal の枠と文字。押しただけでは何も消えない)。
  Widget _dangerButton(
    BuildContext context, {
    required VoidCallback? onPressed,
    required String label,
  }) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return OutlinedButton(
      onPressed: onPressed,
      style: OutlinedButton.styleFrom(
        foregroundColor: brewbook.palette.signal,
        minimumSize: const Size(0, 36),
        padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x4),
        textStyle: AppTextStyle.label(color: brewbook.palette.signal),
        side: BorderSide(color: brewbook.palette.signal),
      ),
      child: Text(label),
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
      return Text(l10n.loading, style: AppTextStyle.body(color: BrewbookTheme.of(context).palette.inkMuted));
    }
    // パスキーが 1 つしかないときは削除できない (設計判断)。サーバーも 409 を返す。
    final canDelete = passkeys.length > 1;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        for (final passkey in passkeys) _passkeyRow(context, l10n, passkey, canDelete),
        const SizedBox(height: AppSpacing.x3),
        Align(
          alignment: Alignment.centerLeft,
          child: _secondaryButton(
            context,
            onPressed: _busy ? null : _addPasskey,
            icon: Icons.add_outlined,
            label: l10n.addPasskeyButton,
          ),
        ),
      ],
    );
  }

  /// パスキーの 1 行 (鍵の印、名前、日時、変更と削除)。
  Widget _passkeyRow(
    BuildContext context,
    AppLocalizations l10n,
    Passkey passkey,
    bool canDelete,
  ) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(vertical: AppSpacing.x3),
      decoration: BoxDecoration(
        border: Border(bottom: BorderSide(color: brewbook.line)),
      ),
      child: Row(
        children: <Widget>[
          Icon(Icons.key_outlined, size: 24, color: brewbook.palette.inkMuted),
          const SizedBox(width: AppSpacing.x3),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  passkey.name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppTextStyle.body(
                    color: brewbook.palette.ink,
                  ).copyWith(fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 2),
                Text(
                  _passkeySubtitle(context, passkey),
                  style: AppTextStyle.mono(
                    size: 12,
                    lineHeight: 16,
                    color: brewbook.palette.inkMuted,
                  ),
                ),
              ],
            ),
          ),
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
    );
  }
}

/// 設定の画面の節 (見出し、説明、操作)。
class _Section extends StatelessWidget {
  const _Section({this.title, this.description, required this.child});

  /// 節の見出し (ARB から取る)。無いときは出さない。
  final String? title;

  /// 節の説明 (ARB から取る)。無いときは出さない。
  final String? description;

  /// 節の中身。
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final String? titleText = title;
    final String? descriptionText = description;
    return Container(
      padding: const EdgeInsets.all(AppSpacing.x4),
      decoration: BoxDecoration(
        border: Border(bottom: BorderSide(color: brewbook.line)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          if (titleText != null)
            Text(titleText, style: AppTextStyle.heading(color: brewbook.palette.ink)),
          if (descriptionText != null) ...<Widget>[
            const SizedBox(height: AppSpacing.x3),
            Text(descriptionText, style: AppTextStyle.body(color: brewbook.palette.inkMuted)),
          ],
          const SizedBox(height: AppSpacing.x3),
          child,
        ],
      ),
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
      content: AppTextField(
        controller: _name,
        label: l10n.passkeyNameLabel,
        enabled: true,
        errorText: _error,
        helperText: l10n.passkeyNameHelper,
        hintText: l10n.passkeyNameHint,
        mono: false,
        autofocus: true,
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
