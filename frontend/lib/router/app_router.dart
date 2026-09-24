import 'package:go_router/go_router.dart';

import '../auth/auth_controller.dart';
import '../screens/home_screen.dart';
import '../screens/login_screen.dart';
import '../screens/register_screen.dart';

/// 画面の経路の台帳 (ADR-0007)。経路の追加は 0014 以降がここに行う。
abstract final class AppRoutes {
  /// ログイン (FR-2)。
  static const String login = '/login';

  /// 登録用トークンによるパスキーの登録 (FR-1)。トークンはクエリで渡す。
  static const String register = '/register';

  /// ホーム (抽出の一覧)。中身は 0014 が作る。
  static const String home = '/';

  /// 登録の画面のトークンのクエリパラメータの名前。
  static const String tokenParameter = 'token';

  /// 登録の画面の経路を、トークンから組み立てる。
  static String registerWithToken(String token) {
    return '$register?$tokenParameter=${Uri.encodeQueryComponent(token)}';
  }
}

/// 経路の台帳からルーターを組み立てる。
///
/// ログインの状態は [AuthController] から取り、`refreshListenable` で状態の変化のたびに
/// 遷移をやり直す。401 はログイン画面へ、ログイン済みのログイン画面と登録の画面はホームへ遷移させる。
GoRouter createAppRouter(AuthController controller) {
  return GoRouter(
    initialLocation: AppRoutes.home,
    refreshListenable: controller,
    redirect: (context, state) {
      final location = state.matchedLocation;
      final onLogin = location == AppRoutes.login;
      final onRegister = location == AppRoutes.register;
      switch (controller.status) {
        case SessionStatus.checking:
        case SessionStatus.unknown:
          // 起動時の確認が終わるまで遷移させない。確認できなかった場合はホームが再試行を促す。
          return null;
        case SessionStatus.signedOut:
          // ログインと登録 (トークンがあれば開ける) 以外はログイン画面へ遷移させる (FR-1)。
          if (onLogin || onRegister) {
            return null;
          }
          return AppRoutes.login;
        case SessionStatus.signedIn:
          // ログイン済みでログインと登録を開いたときはホームへ戻す。
          if (onLogin || onRegister) {
            return AppRoutes.home;
          }
          return null;
      }
    },
    routes: <RouteBase>[
      GoRoute(
        path: AppRoutes.home,
        builder: (context, state) => const HomeScreen(),
      ),
      GoRoute(
        path: AppRoutes.login,
        builder: (context, state) => const LoginScreen(),
      ),
      GoRoute(
        path: AppRoutes.register,
        builder: (context, state) => RegisterScreen(
          token: state.uri.queryParameters[AppRoutes.tokenParameter],
        ),
      ),
    ],
  );
}
