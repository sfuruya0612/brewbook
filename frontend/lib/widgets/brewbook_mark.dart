import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import '../theme/tokens.dart';

/// brewbook の印 (docs/design/assets/Marks/mark.svg)。
///
/// ドリッパーの一滴が記録の行に落ちる形。ログインの画面と、記録が無いときの表示に使う。
/// SVG の座標 (viewBox 100 106 312 368) をそのまま描く。
class BrewbookMark extends StatelessWidget {
  const BrewbookMark({super.key, this.width = 72, this.color});

  /// 印の幅 (px)。高さは縦横比から決まる。
  final double width;

  /// 塗りの色。無いときはテーマの roast を使う。
  final Color? color;

  /// 幅に対する高さの比 (svg の viewBox から)。
  static const double aspectRatio = 312 / 368;

  @override
  Widget build(BuildContext context) {
    final paintColor = color ?? Theme.of(context).colorScheme.primary;
    return CustomPaint(
      size: Size(width, width / aspectRatio),
      painter: _BrewbookMarkPainter(paintColor),
    );
  }
}

class _BrewbookMarkPainter extends CustomPainter {
  const _BrewbookMarkPainter(this.color);

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final scale = size.width / 312;
    canvas.save();
    // viewBox の原点 (100, 106) をキャンバスの原点に合わせる。
    canvas.translate(-100 * scale, -106 * scale);
    canvas.scale(scale);

    final fill = Paint()
      ..color = color
      ..style = PaintingStyle.fill
      ..isAntiAlias = true;
    final stroke = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 8
      ..strokeJoin = StrokeJoin.round;

    // 上の帯。
    canvas.drawRRect(
      RRect.fromRectAndRadius(const Rect.fromLTWH(118, 124, 276, 34), const Radius.circular(17)),
      fill,
    );
    // ドリッパー。
    final dripper = Path()
      ..moveTo(148, 158)
      ..lineTo(364, 158)
      ..lineTo(298, 286)
      ..lineTo(214, 286)
      ..close();
    canvas.drawPath(dripper, fill);
    canvas.drawPath(dripper, stroke);
    // 一滴。
    final drop = Path()
      ..moveTo(256, 298)
      ..cubicTo(256, 298, 228, 332, 228, 352)
      ..arcToPoint(const Offset(284, 352), radius: const Radius.circular(28), clockwise: true)
      ..cubicTo(284, 332, 256, 298, 256, 298)
      ..close();
    canvas.drawPath(drop, fill);
    // 記録の行 (2 本)。
    canvas.drawRRect(
      RRect.fromRectAndRadius(const Rect.fromLTWH(118, 386, 276, 26), const Radius.circular(13)),
      fill,
    );
    canvas.drawRRect(
      RRect.fromRectAndRadius(const Rect.fromLTWH(118, 430, 168, 26), const Radius.circular(13)),
      fill,
    );
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant _BrewbookMarkPainter oldDelegate) => oldDelegate.color != color;
}

/// 記録が無いときの表示 (docs/design/README.md の「状態」)。
///
/// 印を [BrewbookMark] の薄い色で置き、「記録がありません」と、次にすることを 1 文で示す。
class EmptyState extends StatelessWidget {
  const EmptyState({super.key, required this.message, this.hint, this.iconColor});

  /// 主の文言 (ARB から取る)。
  final String message;

  /// 次にすることを示す 1 文 (ARB から取る)。無いときは出さない。
  final String? hint;

  /// 印の色。無いときはテーマの ink-faint を使う。
  final Color? iconColor;

  @override
  Widget build(BuildContext context) {
    final palette = BrewbookTheme.of(context).palette;
    final hintText = hint;
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.x6,
          vertical: AppSpacing.x12,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            BrewbookMark(width: 56, color: iconColor ?? palette.inkFaint.withValues(alpha: 0.6)),
            const SizedBox(height: AppSpacing.x4),
            Text(
              message,
              textAlign: TextAlign.center,
              style: AppTextStyle.body(color: palette.ink),
            ),
            if (hintText != null) ...<Widget>[
              const SizedBox(height: AppSpacing.x2),
              Text(
                hintText,
                textAlign: TextAlign.center,
                style: AppTextStyle.caption(color: palette.inkMuted),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
