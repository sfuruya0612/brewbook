import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';

import 'tokens.dart';

/// デザインのトークンのうち [ColorScheme] に載らないものを運ぶ拡張。
///
/// 色と影の対応は docs/design/README.md の「Flutter への落とし込み」に従う。
/// 画面のコードは `BrewbookTheme.of(context)` から取り、テーマの切り替えに追従させる。
@immutable
class BrewbookTheme extends ThemeExtension<BrewbookTheme> {
  const BrewbookTheme({required this.palette});

  /// このテーマのパレット。
  final BrewbookPalette palette;

  /// アクセントの琥珀色。評価の印とグラムのグラフだけに使う。
  Color get crema => palette.crema;

  /// crema を文字として使うときの色。
  Color get cremaInk => palette.cremaInk;

  /// フレーバーノートのタグの地。
  Color get cremaSoft => palette.cremaSoft;

  /// プレースホルダーと無効化した操作の文字だけに使う色。
  Color get inkFaint => palette.inkFaint;

  /// 罫線の色。
  Color get line => palette.line;

  /// 意味を持つ線の色。
  Color get lineStrong => palette.lineStrong;

  /// 件数と金額の棒の色。
  Color get chartCount => palette.chartCount;

  /// グラムの棒、散布図の点、折れ線の色。
  Color get chartGrams => palette.chartGrams;

  /// 参照先のタイルと選択中の行の地。
  Color get roastSoft => palette.roastSoft;

  /// 浮くもの (FAB、メニュー、ダイアログ、スナックバー) の影。
  List<BoxShadow> get shadowFloat => shadowFloatFor(
    ThemeData.estimateBrightnessForColor(palette.paper),
  );

  /// 環境の [BrewbookTheme]。テストでは必ずテーマを組んで渡す。
  static BrewbookTheme of(BuildContext context) {
    final theme = Theme.of(context).extension<BrewbookTheme>();
    assert(theme != null, 'BrewbookTheme が ThemeData に登録されていません');
    return theme!;
  }

  @override
  BrewbookTheme copyWith({BrewbookPalette? palette}) {
    return BrewbookTheme(palette: palette ?? this.palette);
  }

  @override
  BrewbookTheme lerp(covariant BrewbookTheme? other, double t) {
    if (other == null) {
      return this;
    }
    return t < 0.5 ? this : other;
  }
}

/// 型のトークン (tokens.json の type)。
///
/// 数値と日付は [value]、brewbook の名前だけ [wordmark] を使う。
/// 主題は [ThemeData] の [TextTheme] に載せる。
abstract final class AppTextStyle {
  /// IBM Plex Sans JP が使えない環境でのフォールバック。
  static const List<String> sansFallback = <String>[
    'Hiragino Sans',
    'Noto Sans JP',
    'Noto Sans CJK JP',
  ];

  /// IBM Plex Mono のフォールバック。
  static const List<String> monoFallback = <String>['SF Mono', 'Menlo', 'Consolas'];

  /// IBM Plex Serif のフォールバック。
  static const List<String> serifFallback = <String>['Georgia'];

  /// 本文の族 (IBM Plex Sans JP) で組む。
  static TextStyle sans({
    required double size,
    required double lineHeight,
    FontWeight weight = FontWeight.w400,
    double? letterSpacing,
    Color? color,
  }) {
    return GoogleFonts.ibmPlexSansJp(
      fontSize: size,
      height: lineHeight / size,
      fontWeight: weight,
      letterSpacing: letterSpacing,
      color: color,
    ).copyWith(fontFamilyFallback: sansFallback);
  }

  /// 数値の族 (IBM Plex Mono) で組む。桁を揃える。
  static TextStyle mono({
    required double size,
    required double lineHeight,
    FontWeight weight = FontWeight.w400,
    double? letterSpacing,
    Color? color,
  }) {
    return GoogleFonts.ibmPlexMono(
      fontSize: size,
      height: lineHeight / size,
      fontWeight: weight,
      letterSpacing: letterSpacing,
      color: color,
      fontFeatures: const <FontFeature>[FontFeature.tabularFigures()],
    ).copyWith(fontFamilyFallback: monoFallback);
  }

  /// brewbook の名前だけの族 (IBM Plex Serif)。
  static TextStyle serif({
    required double size,
    required double lineHeight,
    FontWeight weight = FontWeight.w500,
    double? letterSpacing,
    Color? color,
  }) {
    return GoogleFonts.ibmPlexSerif(
      fontSize: size,
      height: lineHeight / size,
      fontWeight: weight,
      letterSpacing: letterSpacing,
      color: color,
    ).copyWith(fontFamilyFallback: serifFallback);
  }

  /// 画面の題 (20 / 28、600)。
  static TextStyle title({Color? color}) =>
      sans(size: 20, lineHeight: 28, weight: FontWeight.w600, color: color);

  /// 見出しと一覧の行の 1 行目 (16 / 24、600)。
  static TextStyle heading({Color? color}) =>
      sans(size: 16, lineHeight: 24, weight: FontWeight.w600, color: color);

  /// 本文 (15 / 22、400)。
  static TextStyle body({Color? color}) => sans(size: 15, lineHeight: 22, color: color);

  /// 項目名、チップ、ボタン (13 / 18、500、字間 0.02em)。
  static TextStyle label({Color? color}) => sans(
    size: 13,
    lineHeight: 18,
    weight: FontWeight.w500,
    letterSpacing: 0.02,
    color: color,
  );

  /// 行の 2 行目、単位、注記 (12 / 16、400)。
  static TextStyle caption({Color? color}) => sans(size: 12, lineHeight: 16, color: color);

  /// 数値と日付 (15 / 22、400)。
  static TextStyle value({Color? color}) => mono(size: 15, lineHeight: 22, color: color);

  /// 統計の合計の値 (28 / 32、500)。
  static TextStyle valueLarge({Color? color}) =>
      mono(size: 28, lineHeight: 32, weight: FontWeight.w500, letterSpacing: -0.28, color: color);

  /// brewbook の名前 (32 / 36、500)。
  static TextStyle wordmark({Color? color}) => serif(
    size: 32,
    lineHeight: 36,
    letterSpacing: -0.32,
    color: color,
  );

  /// AppBar の名前 (24 / 28、500)。
  static TextStyle appBarWordmark({Color? color}) => serif(
    size: 24,
    lineHeight: 28,
    letterSpacing: -0.24,
    color: color,
  );

  /// グラフの軸の目盛 (11 / 16、400)。
  static TextStyle chartAxis({Color? color}) => mono(size: 11, lineHeight: 16, color: color);
}

/// Paper と Night の [ThemeData] を組み立てる。
///
/// 対応は docs/design/README.md の「Flutter への落とし込み」に従う。
ThemeData buildBrewbookTheme(Brightness brightness) {
  final BrewbookPalette palette = paletteFor(brightness);
  final ColorScheme scheme = ColorScheme(
    brightness: brightness,
    primary: palette.roast,
    onPrimary: palette.onRoast,
    primaryContainer: palette.roastSoft,
    onPrimaryContainer: palette.ink,
    secondary: palette.cremaInk,
    onSecondary: palette.paper,
    secondaryContainer: palette.cremaSoft,
    onSecondaryContainer: palette.ink,
    tertiary: palette.crema,
    onTertiary: palette.ink,
    error: palette.signal,
    onError: palette.onSignal,
    errorContainer: palette.signalSoft,
    onErrorContainer: palette.signal,
    surface: palette.paper,
    onSurface: palette.ink,
    surfaceContainerLowest: palette.paperSunken,
    surfaceContainerLow: palette.paperRaised,
    surfaceContainer: palette.paperRaised,
    surfaceContainerHigh: palette.paperRaised,
    surfaceContainerHighest: palette.paperSunken,
    surfaceTint: Colors.transparent,
    onSurfaceVariant: palette.inkMuted,
    outline: palette.lineStrong,
    outlineVariant: palette.line,
    shadow: palette.ink,
    scrim: palette.ink,
    inverseSurface: palette.ink,
    onInverseSurface: palette.paper,
    inversePrimary: palette.roastSoft,
    surfaceDim: palette.paper,
    surfaceBright: palette.paperRaised,
  );

  final TextTheme textTheme = TextTheme(
    displayLarge: AppTextStyle.title(color: palette.ink),
    displayMedium: AppTextStyle.title(color: palette.ink),
    displaySmall: AppTextStyle.title(color: palette.ink),
    headlineLarge: AppTextStyle.title(color: palette.ink),
    headlineMedium: AppTextStyle.title(color: palette.ink),
    headlineSmall: AppTextStyle.heading(color: palette.ink),
    titleLarge: AppTextStyle.title(color: palette.ink),
    titleMedium: AppTextStyle.heading(color: palette.ink),
    titleSmall: AppTextStyle.label(color: palette.ink),
    bodyLarge: AppTextStyle.body(color: palette.ink),
    bodyMedium: AppTextStyle.body(color: palette.ink),
    bodySmall: AppTextStyle.caption(color: palette.inkMuted),
    labelLarge: AppTextStyle.body(color: palette.ink),
    labelMedium: AppTextStyle.label(color: palette.ink),
    labelSmall: AppTextStyle.caption(color: palette.inkMuted),
  );

  final Color disabledFill = palette.paperSunken;
  const WidgetStateProperty<double> noElevation = WidgetStatePropertyAll<double>(0);

  return ThemeData(
    useMaterial3: true,
    brightness: brightness,
    colorScheme: scheme,
    scaffoldBackgroundColor: palette.paper,
    canvasColor: palette.paper,
    textTheme: textTheme,
    visualDensity: VisualDensity.standard,
    // 画面内のアイコンは outlined の 24 px だけを使う。
    iconTheme: IconThemeData(color: palette.ink, size: 24),
    dividerTheme: DividerThemeData(color: palette.line, thickness: 1, space: 1),
    appBarTheme: AppBarTheme(
      backgroundColor: palette.paper,
      foregroundColor: palette.ink,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
      toolbarHeight: 56,
      titleTextStyle: AppTextStyle.title(color: palette.ink),
      iconTheme: IconThemeData(color: palette.ink, size: 24),
      actionsIconTheme: IconThemeData(color: palette.ink, size: 24),
      // 下端に line の罫線を引く (影は付けない)。
      shape: Border(bottom: BorderSide(color: palette.line)),
    ),
    actionIconTheme: ActionIconThemeData(
      // 戻るは「<」の線、閉じるは「x」の線にする (AppBar のガイドライン)。
      backButtonIconBuilder: (context) => const Icon(Icons.arrow_back_ios_new_outlined),
      closeButtonIconBuilder: (context) => const Icon(Icons.close_outlined),
    ),
    inputDecorationTheme: InputDecorationThemeData(
      filled: true,
      fillColor: palette.paperSunken,
      contentPadding: const EdgeInsets.symmetric(horizontal: AppSpacing.x3, vertical: AppSpacing.x3),
      hintStyle: AppTextStyle.body(color: palette.inkFaint),
      errorStyle: AppTextStyle.caption(color: palette.signal),
      helperStyle: AppTextStyle.caption(color: palette.inkMuted),
      suffixIconColor: palette.inkMuted,
      prefixIconColor: palette.inkMuted,
      border: OutlineInputBorder(
        borderRadius: AppRadius.smAll,
        borderSide: BorderSide(color: palette.lineStrong),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: AppRadius.smAll,
        borderSide: BorderSide(color: palette.lineStrong),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: AppRadius.smAll,
        // フォーカスは枠を crema-ink にする。デザインの focus-ring (2 px の paper の
        // 隙間の外の 2 px の crema-ink) は InputDecoration では表せないため、枠の色で示す。
        borderSide: BorderSide(color: palette.cremaInk),
      ),
      errorBorder: OutlineInputBorder(
        borderRadius: AppRadius.smAll,
        borderSide: BorderSide(color: palette.signal),
      ),
      focusedErrorBorder: OutlineInputBorder(
        borderRadius: AppRadius.smAll,
        borderSide: BorderSide(color: palette.signal),
      ),
      disabledBorder: OutlineInputBorder(
        borderRadius: AppRadius.smAll,
        borderSide: BorderSide(color: palette.line),
      ),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: ButtonStyle(
        backgroundColor: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.disabled)) {
            return disabledFill;
          }
          return palette.roast;
        }),
        foregroundColor: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.disabled)) {
            return palette.inkFaint;
          }
          return palette.onRoast;
        }),
        textStyle: WidgetStatePropertyAll<TextStyle>(AppTextStyle.body(color: palette.onRoast)),
        minimumSize: const WidgetStatePropertyAll<Size>(Size(0, AppSpacing.x12)),
        padding: const WidgetStatePropertyAll<EdgeInsetsGeometry>(
          EdgeInsets.symmetric(horizontal: AppSpacing.x6),
        ),
        shape: const WidgetStatePropertyAll<OutlinedBorder>(
          RoundedRectangleBorder(borderRadius: AppRadius.mdAll),
        ),
        elevation: noElevation,
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: ButtonStyle(
        backgroundColor: const WidgetStatePropertyAll<Color>(Colors.transparent),
        foregroundColor: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.disabled)) {
            return palette.inkFaint;
          }
          return palette.ink;
        }),
        side: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.disabled)) {
            return BorderSide(color: palette.line);
          }
          return BorderSide(color: palette.lineStrong);
        }),
        textStyle: WidgetStatePropertyAll<TextStyle>(AppTextStyle.body(color: palette.ink)),
        minimumSize: const WidgetStatePropertyAll<Size>(Size(0, AppSpacing.x12)),
        padding: const WidgetStatePropertyAll<EdgeInsetsGeometry>(
          EdgeInsets.symmetric(horizontal: AppSpacing.x6),
        ),
        shape: const WidgetStatePropertyAll<OutlinedBorder>(
          RoundedRectangleBorder(borderRadius: AppRadius.mdAll),
        ),
        elevation: noElevation,
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: ButtonStyle(
        foregroundColor: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.disabled)) {
            return palette.inkFaint;
          }
          return palette.cremaInk;
        }),
        textStyle: WidgetStatePropertyAll<TextStyle>(AppTextStyle.body(color: palette.cremaInk)),
        minimumSize: const WidgetStatePropertyAll<Size>(Size(0, 40)),
        padding: const WidgetStatePropertyAll<EdgeInsetsGeometry>(
          EdgeInsets.symmetric(horizontal: AppSpacing.x3),
        ),
        shape: const WidgetStatePropertyAll<OutlinedBorder>(
          RoundedRectangleBorder(borderRadius: AppRadius.mdAll),
        ),
      ),
    ),
    iconButtonTheme: IconButtonThemeData(
      style: ButtonStyle(
        foregroundColor: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.disabled)) {
            return palette.inkFaint;
          }
          return palette.ink;
        }),
        iconSize: const WidgetStatePropertyAll<double>(24),
        minimumSize: const WidgetStatePropertyAll<Size>(Size(40, 40)),
        padding: const WidgetStatePropertyAll<EdgeInsetsGeometry>(EdgeInsets.all(AppSpacing.x2)),
        shape: const WidgetStatePropertyAll<OutlinedBorder>(
          RoundedRectangleBorder(borderRadius: AppRadius.fullAll),
        ),
      ),
    ),
    floatingActionButtonTheme: FloatingActionButtonThemeData(
      backgroundColor: palette.roast,
      foregroundColor: palette.onRoast,
      // 浮くものだけに shadow-float を使う。Flutter の elevation は影の形を細かく
      // 指定できないため、近い elevation 3 で表す。
      elevation: 3,
      focusElevation: 3,
      hoverElevation: 3,
      highlightElevation: 3,
      shape: const RoundedRectangleBorder(borderRadius: AppRadius.lgAll),
      extendedTextStyle: AppTextStyle.body(color: palette.onRoast).copyWith(
        fontWeight: FontWeight.w500,
      ),
    ),
    snackBarTheme: SnackBarThemeData(
      backgroundColor: palette.ink,
      contentTextStyle: AppTextStyle.body(color: palette.paper),
      actionTextColor: palette.crema,
      behavior: SnackBarBehavior.floating,
      elevation: 3,
      shape: const RoundedRectangleBorder(borderRadius: AppRadius.mdAll),
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: palette.paperRaised,
      surfaceTintColor: Colors.transparent,
      elevation: 3,
      shadowColor: palette.ink.withValues(alpha: 0.3),
      shape: const RoundedRectangleBorder(borderRadius: AppRadius.mdAll),
      insetPadding: const EdgeInsets.all(AppSpacing.x6),
      titleTextStyle: AppTextStyle.title(color: palette.ink),
      contentTextStyle: AppTextStyle.body(color: palette.inkMuted),
    ),
    popupMenuTheme: PopupMenuThemeData(
      color: palette.paperRaised,
      surfaceTintColor: Colors.transparent,
      elevation: 3,
      shape: RoundedRectangleBorder(
        borderRadius: AppRadius.mdAll,
        side: BorderSide(color: palette.line),
      ),
      textStyle: AppTextStyle.body(color: palette.ink),
      labelTextStyle: WidgetStatePropertyAll<TextStyle>(AppTextStyle.body(color: palette.ink)),
    ),
    bottomSheetTheme: BottomSheetThemeData(
      backgroundColor: palette.paperRaised,
      surfaceTintColor: Colors.transparent,
      modalBackgroundColor: palette.paperRaised,
      elevation: 3,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(AppRadius.lg)),
      ),
    ),
    chipTheme: ChipThemeData(
      backgroundColor: Colors.transparent,
      selectedColor: palette.roast,
      secondarySelectedColor: palette.cremaSoft,
      disabledColor: palette.paperSunken,
      checkmarkColor: palette.onRoast,
      labelStyle: AppTextStyle.label(color: palette.ink),
      secondaryLabelStyle: AppTextStyle.label(color: palette.onRoast),
      side: BorderSide(color: palette.lineStrong),
      shape: const StadiumBorder(),
      padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x3),
      showCheckmark: false,
    ),
    switchTheme: SwitchThemeData(
      thumbColor: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.disabled)) {
          return palette.inkFaint;
        }
        if (states.contains(WidgetState.selected)) {
          return palette.onRoast;
        }
        return palette.lineStrong;
      }),
      trackColor: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.disabled)) {
          return palette.paperSunken;
        }
        if (states.contains(WidgetState.selected)) {
          return palette.roast;
        }
        return Colors.transparent;
      }),
      trackOutlineColor: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.selected)) {
          return palette.roast;
        }
        return palette.lineStrong;
      }),
      trackOutlineWidth: const WidgetStatePropertyAll<double>(1),
    ),
    listTileTheme: const ListTileThemeData(
      minTileHeight: 64,
      contentPadding: EdgeInsets.symmetric(horizontal: AppSpacing.x4),
    ),
    progressIndicatorTheme: ProgressIndicatorThemeData(color: palette.roast),
    textSelectionTheme: TextSelectionThemeData(
      cursorColor: palette.cremaInk,
      selectionColor: palette.cremaSoft,
      selectionHandleColor: palette.cremaInk,
    ),
    tooltipTheme: TooltipThemeData(
      decoration: BoxDecoration(
        color: palette.paperRaised,
        borderRadius: AppRadius.smAll,
        border: Border.all(color: palette.line),
      ),
      textStyle: AppTextStyle.caption(color: palette.ink),
      waitDuration: const Duration(milliseconds: 500),
    ),
    extensions: <ThemeExtension<dynamic>>[BrewbookTheme(palette: palette)],
  );
}
