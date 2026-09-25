import 'package:flutter/widgets.dart';
import 'package:go_router/go_router.dart';

import '../auth/auth_controller.dart';
import '../records/record_services.dart';
import '../screens/brew_detail_screen.dart';
import '../screens/brew_form_screen.dart';
import '../screens/home_screen.dart';
import '../screens/login_screen.dart';
import '../screens/product_form_screen.dart';
import '../screens/product_list_screen.dart';
import '../screens/purchase_detail_screen.dart';
import '../screens/purchase_form_screen.dart';
import '../screens/purchase_list_screen.dart';
import '../screens/register_screen.dart';
import '../screens/shop_form_screen.dart';
import '../screens/shop_list_screen.dart';
import '../screens/stats_screen.dart';

/// 画面の経路の台帳 (ADR-0007)。
///
/// 経路の名前は経路のパターンと同じにし、画面数の成功指標 (PRD の成功指標) で
/// 「ルーターに登録した画面の種類」を数えられるようにする。
abstract final class AppRoutes {
  /// ログイン (FR-2)。
  static const String login = '/login';

  /// 登録用トークンによるパスキーの登録 (FR-1)。トークンはクエリで渡す。
  static const String register = '/register';

  /// ホーム (抽出の一覧。FR-11)。
  static const String home = '/';

  /// 抽出の登録 (FR-11)。
  static const String brewNew = '/brews/new';

  /// 抽出の詳細 (FR-11)。
  static const String brewDetail = '/brews/:id';

  /// 抽出の編集 (FR-11)。
  static const String brewEdit = '/brews/:id/edit';

  /// 購入の一覧 (FR-9)。
  static const String purchases = '/purchases';

  /// 購入の登録 (FR-9)。
  static const String purchaseNew = '/purchases/new';

  /// 購入の詳細 (FR-9、FR-10、FR-18)。
  static const String purchaseDetail = '/purchases/:id';

  /// 購入の編集 (FR-9)。
  static const String purchaseEdit = '/purchases/:id/edit';

  /// 商品の一覧 (FR-7)。
  static const String products = '/products';

  /// 商品の登録 (FR-7)。
  static const String productNew = '/products/new';

  /// 商品の編集 (FR-7)。
  static const String productEdit = '/products/:id/edit';

  /// 店の一覧 (FR-6)。
  static const String shops = '/shops';

  /// 店の登録 (FR-6)。
  static const String shopNew = '/shops/new';

  /// 店の編集 (FR-6)。
  static const String shopEdit = '/shops/:id/edit';

  /// 統計 (FR-18)。
  static const String stats = '/stats';

  /// 登録の画面のトークンのクエリパラメータの名前。
  static const String tokenParameter = 'token';

  /// 登録の画面の経路を、トークンから組み立てる。
  static String registerWithToken(String token) {
    return '$register?$tokenParameter=${Uri.encodeQueryComponent(token)}';
  }

  /// 抽出の詳細の経路。
  static String brewPath(String id) => '/brews/${Uri.encodeComponent(id)}';

  /// 抽出の編集の経路。
  static String brewEditPath(String id) => '/brews/${Uri.encodeComponent(id)}/edit';

  /// 購入の詳細の経路。
  static String purchasePath(String id) => '/purchases/${Uri.encodeComponent(id)}';

  /// 購入の編集の経路。
  static String purchaseEditPath(String id) => '/purchases/${Uri.encodeComponent(id)}/edit';

  /// 商品の編集の経路。
  static String productEditPath(String id) => '/products/${Uri.encodeComponent(id)}/edit';

  /// 店の編集の経路。
  static String shopEditPath(String id) => '/shops/${Uri.encodeComponent(id)}/edit';
}

/// 経路の台帳からルーターを組み立てる。
///
/// ログインの状態は [AuthController] から取り、`refreshListenable` で状態の変化のたびに
/// 遷移をやり直す。401 はログイン画面へ、ログイン済みのログイン画面と登録の画面はホームへ遷移させる。
/// [observers] は画面数の成功指標 (PRD の成功指標) を測るテストが渡す。
GoRouter createAppRouter(
  AuthController controller,
  RecordServices services, {
  List<NavigatorObserver> observers = const <NavigatorObserver>[],
}) {
  return GoRouter(
    initialLocation: AppRoutes.home,
    refreshListenable: controller,
    observers: observers,
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
        name: AppRoutes.home,
        builder: (context, state) => HomeScreen(services: services),
      ),
      GoRoute(
        path: AppRoutes.login,
        name: AppRoutes.login,
        builder: (context, state) => const LoginScreen(),
      ),
      GoRoute(
        path: AppRoutes.register,
        name: AppRoutes.register,
        builder: (context, state) => RegisterScreen(
          token: state.uri.queryParameters[AppRoutes.tokenParameter],
        ),
      ),
      GoRoute(
        path: AppRoutes.brewNew,
        name: AppRoutes.brewNew,
        builder: (context, state) => BrewFormScreen(services: services),
      ),
      GoRoute(
        path: AppRoutes.brewDetail,
        name: AppRoutes.brewDetail,
        builder: (context, state) =>
            BrewDetailScreen(services: services, id: state.pathParameters['id']!),
      ),
      GoRoute(
        path: AppRoutes.brewEdit,
        name: AppRoutes.brewEdit,
        builder: (context, state) =>
            BrewFormScreen(services: services, id: state.pathParameters['id']!),
      ),
      GoRoute(
        path: AppRoutes.purchases,
        name: AppRoutes.purchases,
        builder: (context, state) => PurchaseListScreen(services: services),
      ),
      GoRoute(
        path: AppRoutes.purchaseNew,
        name: AppRoutes.purchaseNew,
        builder: (context, state) => PurchaseFormScreen(services: services),
      ),
      GoRoute(
        path: AppRoutes.purchaseDetail,
        name: AppRoutes.purchaseDetail,
        builder: (context, state) =>
            PurchaseDetailScreen(services: services, id: state.pathParameters['id']!),
      ),
      GoRoute(
        path: AppRoutes.purchaseEdit,
        name: AppRoutes.purchaseEdit,
        builder: (context, state) =>
            PurchaseFormScreen(services: services, id: state.pathParameters['id']!),
      ),
      GoRoute(
        path: AppRoutes.products,
        name: AppRoutes.products,
        builder: (context, state) => ProductListScreen(services: services),
      ),
      GoRoute(
        path: AppRoutes.productNew,
        name: AppRoutes.productNew,
        builder: (context, state) => ProductFormScreen(services: services),
      ),
      GoRoute(
        path: AppRoutes.productEdit,
        name: AppRoutes.productEdit,
        builder: (context, state) =>
            ProductFormScreen(services: services, id: state.pathParameters['id']!),
      ),
      GoRoute(
        path: AppRoutes.shops,
        name: AppRoutes.shops,
        builder: (context, state) => ShopListScreen(services: services),
      ),
      GoRoute(
        path: AppRoutes.shopNew,
        name: AppRoutes.shopNew,
        builder: (context, state) => ShopFormScreen(services: services),
      ),
      GoRoute(
        path: AppRoutes.shopEdit,
        name: AppRoutes.shopEdit,
        builder: (context, state) =>
            ShopFormScreen(services: services, id: state.pathParameters['id']!),
      ),
      GoRoute(
        path: AppRoutes.stats,
        name: AppRoutes.stats,
        builder: (context, state) => StatsScreen(services: services),
      ),
    ],
  );
}
