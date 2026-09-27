import 'package:flutter/material.dart';

import '../api/api_error.dart';
import '../auth/auth_scope.dart';
import '../auth/passkey_name.dart';
import '../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/app_field.dart';
import '../widgets/app_form.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';

/// 登録用トークンによるパスキーの登録の画面 (FR-1)。
///
/// トークンは `/register?token=<トークン>` のクエリで受け取る。登録が成功するとセッションが
/// 発行され、ルーターがホームへ遷移させる。トークンの 404、409、410 はバナーに文言を出し、
/// フォームを出さずに再発行の案内を続ける。
class RegisterScreen extends StatefulWidget {
  const RegisterScreen({super.key, required this.token});

  /// URL の `token` クエリの値。無ければ null。
  final String? token;

  @override
  State<RegisterScreen> createState() => _RegisterScreenState();
}

class _RegisterScreenState extends State<RegisterScreen> {
  final TextEditingController _name = TextEditingController();

  String? _nameError;
  String? _errorMessage;

  /// 登録用のリンクが無効か (404、409、410)。フォームを出さない。
  bool _tokenInvalid = false;
  bool _busy = false;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  Future<void> _register() async {
    final l10n = AppLocalizations.of(context);
    final controller = AuthScope.read(context);
    final token = widget.token;
    final name = passkeyNameForRequest(_name.text);
    final nameError = validatePasskeyName(_name.text, l10n);
    setState(() {
      _nameError = nameError;
      _errorMessage = null;
    });
    if (token == null || token.isEmpty || nameError != null) {
      return;
    }
    setState(() => _busy = true);
    try {
      await controller.register(token: token, name: name);
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = registerErrorMessage(error, l10n);
        _tokenInvalid = error is ApiError && const <int>{404, 409, 410}.contains(error.status);
      });
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final token = widget.token;
    final bool tokenMissing = token == null || token.isEmpty;
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.registerTitle),
        automaticallyImplyLeading: false,
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(
          AppSpacing.x4,
          AppSpacing.x4,
          AppSpacing.x4,
          AppSpacing.x8,
        ),
        child: tokenMissing || _tokenInvalid
            ? Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  ErrorBanner(message: tokenMissing ? l10n.registerTokenMissing : _errorMessage!),
                  const SizedBox(height: AppSpacing.x5),
                  Text(
                    l10n.registerTokenGuidance,
                    style: AppTextStyle.body(color: brewbook.palette.inkMuted),
                  ),
                ],
              )
            : AppForm(
                children: <Widget>[
                  Text(
                    l10n.registerDescription,
                    style: AppTextStyle.body(color: brewbook.palette.inkMuted),
                  ),
                  AppTextField(
                    controller: _name,
                    label: l10n.passkeyNameLabel,
                    required: true,
                    enabled: !_busy,
                    errorText: _nameError,
                    helperText: l10n.passkeyNameHelper,
                    hintText: l10n.passkeyNameHint,
                    mono: false,
                    onSubmitted: (_) => _register(),
                  ),
                  FilledButton(
                    onPressed: _busy ? null : _register,
                    child: Text(l10n.registerButton),
                  ),
                  if (_errorMessage != null) ErrorBanner(message: _errorMessage!),
                ],
              ),
      ),
    );
  }
}
