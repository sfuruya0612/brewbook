import 'package:flutter/widgets.dart';

import 'auth_controller.dart';

/// [AuthController] をウィジェットツリーに配る。
///
/// 状態管理は Flutter 標準の [ChangeNotifier] だけを使う (ADR-0007) ため、その共有は
/// [InheritedNotifier] で行う。通知があると、購読しているウィジェットが再構築される。
class AuthScope extends InheritedNotifier<AuthController> {
  const AuthScope({super.key, required AuthController controller, required super.child})
    : super(notifier: controller);

  /// [AuthController] を取り出し、変更を購読する。
  static AuthController of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<AuthScope>();
    assert(scope != null, 'AuthScope is not in the widget tree');
    return scope!.notifier!;
  }

  /// 変更を購読せずに [AuthController] を取り出す。イベントハンドラから使う。
  static AuthController read(BuildContext context) {
    final scope = context.getInheritedWidgetOfExactType<AuthScope>();
    assert(scope != null, 'AuthScope is not in the widget tree');
    return scope!.notifier!;
  }
}
