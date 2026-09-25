import 'package:flutter/material.dart';

import '../auth/auth_scope.dart';
import '../auth/passkey_name.dart';
import '../l10n/app_localizations.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';

/// 登録用トークンによるパスキーの登録の画面 (FR-1)。
///
/// トークンは `/register?token=<トークン>` のクエリで受け取る。登録が成功するとセッションが
/// 発行され、ルーターがホームへ遷移させる。
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
      setState(() => _errorMessage = registerErrorMessage(error, l10n));
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final token = widget.token;
    return Scaffold(
      appBar: AppBar(title: Text(l10n.registerTitle)),
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: token == null || token.isEmpty
              ? ErrorBanner(message: l10n.registerTokenMissing)
              : Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    Text(l10n.registerDescription),
                    const SizedBox(height: 16),
                    TextField(
                      controller: _name,
                      enabled: !_busy,
                      decoration: InputDecoration(
                        labelText: l10n.passkeyNameLabel,
                        hintText: l10n.passkeyNameHint,
                        helperText: l10n.passkeyNameHelper,
                        errorText: _nameError,
                      ),
                      onSubmitted: (_) => _register(),
                    ),
                    const SizedBox(height: 24),
                    FilledButton(
                      onPressed: _busy ? null : _register,
                      child: Text(l10n.registerButton),
                    ),
                    if (_busy) ...<Widget>[
                      const SizedBox(height: 16),
                      const Center(child: CircularProgressIndicator()),
                    ],
                    if (_errorMessage != null) ...<Widget>[
                      const SizedBox(height: 16),
                      ErrorBanner(message: _errorMessage!),
                    ],
                  ],
                ),
        ),
      ),
    );
  }
}
