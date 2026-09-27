/// デザインのトークン (docs/design/tokens.json)。
///
/// 画面のコードはこのクラスを直接参照せず、[ThemeData] と `BrewbookTheme` の拡張から
/// 取る。テーマ (Paper と Night) の切り替えに追従させるためである。
/// 4 px を基準にした余白、角丸、罫線の太さだけをここに置く。
library;

import 'package:flutter/material.dart';

/// 地、文字、ブランド色、罫線の 1 テーマ分の値。
class BrewbookPalette {
  const BrewbookPalette({
    required this.paper,
    required this.paperRaised,
    required this.paperSunken,
    required this.line,
    required this.lineStrong,
    required this.ink,
    required this.inkMuted,
    required this.inkFaint,
    required this.roast,
    required this.onRoast,
    required this.roastSoft,
    required this.crema,
    required this.cremaInk,
    required this.cremaSoft,
    required this.signal,
    required this.signalSoft,
    required this.onSignal,
  });

  /// 画面の地。クラフト紙の色。
  final Color paper;

  /// 一覧の行、カード、シート、ダイアログ、メニューの地。
  final Color paperRaised;

  /// 入力欄の地、写真の枠、バッジの地。
  final Color paperSunken;

  /// 罫線。装飾の線なので意味は持たせない。
  final Color line;

  /// 意味を持つ線。入力欄の枠、チップの枠、グラフの軸。
  final Color lineStrong;

  /// 本文と数値の文字。
  final Color ink;

  /// 補足の文字。項目名、単位、キャプション。
  final Color inkMuted;

  /// プレースホルダーと無効化した操作の文字だけ。
  final Color inkFaint;

  /// ブランドの濃いコーヒー色。主要な操作の塗りと、件数と金額の棒。
  final Color roast;

  /// [roast] の塗りの上の文字とアイコン。
  final Color onRoast;

  /// [roast] の薄い地。参照先のタイル、選択中の行。
  final Color roastSoft;

  /// アクセントの琥珀色。塗りとしてだけ使う。
  final Color crema;

  /// [crema] を文字として使うときの色。リンク、評価の数字。
  final Color cremaInk;

  /// [crema] の薄い地。フレーバーノートのタグ、選択中の期間のチップ。
  final Color cremaSoft;

  /// エラーと取り消せない操作。
  final Color signal;

  /// エラーのバナーの地。
  final Color signalSoft;

  /// [signal] の塗りの上の文字。
  final Color onSignal;

  /// 文字用の [roast]。テストと画面のコードの可読性のために別名を置く。
  Color get link => cremaInk;

  /// 件数と金額の棒の色 (roast の別名)。
  Color get chartCount => roast;

  /// グラムの棒、散布図の点、折れ線の色 (crema の別名)。
  Color get chartGrams => crema;
}

/// Paper (ライト) の値。
const BrewbookPalette paperPalette = BrewbookPalette(
  paper: Color(0xfff4ede2),
  paperRaised: Color(0xfffcf8f1),
  paperSunken: Color(0xffeae0d1),
  line: Color(0xffd6c8b4),
  lineStrong: Color(0xff94826a),
  ink: Color(0xff2b1d13),
  inkMuted: Color(0xff6a5847),
  inkFaint: Color(0xff6f5f4e),
  roast: Color(0xff4a2f1c),
  onRoast: Color(0xfff7efe3),
  roastSoft: Color(0xffe6d6c2),
  crema: Color(0xffb8742a),
  cremaInk: Color(0xff8a5313),
  cremaSoft: Color(0xfff3e0c4),
  signal: Color(0xffa8392a),
  signalSoft: Color(0xfff5dcd5),
  onSignal: Color(0xfffdf6f3),
);

/// Night (ダーク) の値。
const BrewbookPalette nightPalette = BrewbookPalette(
  paper: Color(0xff1b1511),
  paperRaised: Color(0xff241d17),
  paperSunken: Color(0xff130f0c),
  line: Color(0xff3a3027),
  lineStrong: Color(0xff7f7060),
  ink: Color(0xfff1e7da),
  inkMuted: Color(0xffb6a692),
  inkFaint: Color(0xff9a8b7a),
  roast: Color(0xffdbb890),
  onRoast: Color(0xff1b1511),
  roastSoft: Color(0xff3a2c20),
  crema: Color(0xffe3a45c),
  cremaInk: Color(0xffe3a45c),
  cremaSoft: Color(0xff3d2a14),
  signal: Color(0xfff0917f),
  signalSoft: Color(0xff4a1f18),
  onSignal: Color(0xff1b1511),
);

/// テーマの明暗から対応するパレットを返す。
BrewbookPalette paletteFor(Brightness brightness) {
  return brightness == Brightness.dark ? nightPalette : paperPalette;
}

/// 4 px を基準にした 8 段の余白 (tokens.json の spacing)。
abstract final class AppSpacing {
  /// アイコンと文字の間、単位と数値の間。
  static const double x1 = 4;

  /// チップの左右の内側、行の中の縦の間隔。
  static const double x2 = 8;

  /// 入力欄の内側、表の行の上下。
  static const double x3 = 12;

  /// 画面の左右の余白、一覧の行の内側、カードの内側。
  static const double x4 = 16;

  /// 入力欄と入力欄の間。
  static const double x5 = 20;

  /// セクションの間、ダイアログの内側。
  static const double x6 = 24;

  /// 画面の題と最初の内容の間、統計のグラフの間。
  static const double x8 = 32;

  /// ログイン画面の名前と本文の間、ボタンの高さ。
  static const double x12 = 48;
}

/// 角の丸み (tokens.json の radius)。
abstract final class AppRadius {
  /// 入力欄、バッジ、写真の枠。
  static const double sm = 4;

  /// ボタン、カード、参照先のタイル、ダイアログ。
  static const double md = 8;

  /// ボトムシートの上の角、拡張 FAB。
  static const double lg = 16;

  /// チップ、評価の印、アバター。
  static const double full = 9999;

  /// [sm] の角丸。
  static const BorderRadius smAll = BorderRadius.all(Radius.circular(sm));

  /// [md] の角丸。
  static const BorderRadius mdAll = BorderRadius.all(Radius.circular(md));

  /// [lg] の角丸。
  static const BorderRadius lgAll = BorderRadius.all(Radius.circular(lg));

  /// [full] の角丸。
  static const BorderRadius fullAll = BorderRadius.all(Radius.circular(full));
}

/// 浮くものだけに使う影 (tokens.json の shadow-float)。
const List<BoxShadow> shadowFloatLight = <BoxShadow>[
  BoxShadow(color: Color(0x262b1d13), blurRadius: 6, offset: Offset(0, 2)),
  BoxShadow(color: Color(0x332b1d13), blurRadius: 24, offset: Offset(0, 8), spreadRadius: -4),
];

/// Night の shadow-float。
const List<BoxShadow> shadowFloatNight = <BoxShadow>[
  BoxShadow(color: Color(0x80000000), blurRadius: 6, offset: Offset(0, 2)),
  BoxShadow(color: Color(0xb3000000), blurRadius: 24, offset: Offset(0, 8), spreadRadius: -4),
];

/// 明暗に対応する shadow-float を返す。
List<BoxShadow> shadowFloatFor(Brightness brightness) {
  return brightness == Brightness.dark ? shadowFloatNight : shadowFloatLight;
}
