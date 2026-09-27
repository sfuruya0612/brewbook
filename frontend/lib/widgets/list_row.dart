import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';

/// 一覧の行 (docs/design/components/ListRow)。
///
/// 1 行目は名前 (heading)、2 行目は日付 (mono) と参照先 (caption、ink-muted)、
/// 右端は数値か評価。高さは 64 px 以上、下端に line の罫線を引く。
class ListRow extends StatelessWidget {
  const ListRow({
    super.key,
    this.leading,
    required this.title,
    this.subtitle,
    this.trailing,
    this.tags,
    this.onTap,
    this.archived = false,
    this.selected = false,
  });

  /// 行の先頭 (写真など)。無いときは置かない。
  final Widget? leading;

  /// 1 行目の名前。
  final String title;

  /// 2 行目。日付は [RowValueText]、参照先は caption で組む。
  final Widget? subtitle;

  /// 右端の数値か評価。
  final Widget? trailing;

  /// 2 行目の下のタグ (商品の行だけ)。
  final List<Widget>? tags;

  /// 押したときの動き。無いときは押せない。
  final VoidCallback? onTap;

  /// アーカイブ済みか。文字を ink-muted に落とし、バッジを付ける。
  final bool archived;

  /// 広い画面で選択中か。地を roast-soft にする。
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final Widget? trailingWidget = trailing;
    final List<Widget>? tagChips = tags;
    final Color titleColor = archived ? brewbook.palette.inkMuted : brewbook.palette.ink;
    final Widget row = Container(
      constraints: const BoxConstraints(minHeight: 64),
      padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x4, vertical: AppSpacing.x3),
      decoration: BoxDecoration(
        color: selected ? brewbook.roastSoft : Colors.transparent,
        border: Border(bottom: BorderSide(color: brewbook.line)),
      ),
      child: Row(
        children: <Widget>[
          ?leading,
          if (leading != null) const SizedBox(width: AppSpacing.x3),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppTextStyle.heading(color: titleColor),
                ),
                if (subtitle != null) ...<Widget>[
                  const SizedBox(height: 2),
                  DefaultTextStyle(
                    style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
                    child: subtitle!,
                  ),
                ],
                if (tagChips != null && tagChips.isNotEmpty) ...<Widget>[
                  const SizedBox(height: AppSpacing.x1 + 2),
                  Wrap(spacing: AppSpacing.x2, runSpacing: AppSpacing.x1, children: tagChips),
                ],
              ],
            ),
          ),
          if (trailingWidget != null || archived) ...<Widget>[
            const SizedBox(width: AppSpacing.x3),
            Column(
              crossAxisAlignment: CrossAxisAlignment.end,
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                ?trailingWidget,
                if (archived) ...<Widget>[
                  if (trailingWidget != null) const SizedBox(height: AppSpacing.x1),
                  const ArchivedBadge(),
                ],
              ],
            ),
          ] else
            const SizedBox(width: AppSpacing.x1),
        ],
      ),
    );
    if (onTap == null) {
      return row;
    }
    return Material(
      color: Colors.transparent,
      child: InkWell(onTap: onTap, child: row),
    );
  }
}

/// 一覧の行の 2 行目の日付など、等幅で組む値。
class RowValueText extends StatelessWidget {
  const RowValueText({super.key, required this.text});

  /// 値。
  final String text;

  @override
  Widget build(BuildContext context) {
    return Text(
      text,
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
      style: AppTextStyle.mono(
        size: 12,
        lineHeight: 16,
        color: BrewbookTheme.of(context).palette.inkMuted,
      ),
    );
  }
}

/// 44 px の写真の枠。写真が無いときはカメラの印を出す。
class ListThumb extends StatelessWidget {
  const ListThumb({super.key, this.image});

  /// 写真。無いときはカメラの印にする。
  final ImageProvider? image;

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final ImageProvider? photo = image;
    return ClipRRect(
      borderRadius: AppRadius.smAll,
      child: Container(
        width: 44,
        height: 44,
        color: brewbook.palette.paperSunken,
        alignment: Alignment.center,
        child: photo == null
            ? Icon(Icons.photo_camera_outlined, size: 22, color: brewbook.inkFaint)
            : Image(image: photo, fit: BoxFit.cover, width: 44, height: 44),
      ),
    );
  }
}

/// 一覧と詳細の中の小さなタグ (高さ 22 px、文字 12 px)。
class TagChip extends StatelessWidget {
  const TagChip({super.key, required this.label});

  /// タグの文字。
  final String label;

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Container(
      height: 22,
      padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x2),
      decoration: BoxDecoration(color: brewbook.cremaSoft, borderRadius: AppRadius.fullAll),
      alignment: Alignment.center,
      child: Text(
        label,
        style: AppTextStyle.caption(color: brewbook.palette.ink),
      ),
    );
  }
}

/// 「アーカイブ済み」のバッジ。
class ArchivedBadge extends StatelessWidget {
  const ArchivedBadge({super.key});

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Container(
      height: 20,
      padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x2),
      decoration: BoxDecoration(
        color: brewbook.palette.paperSunken,
        borderRadius: AppRadius.smAll,
      ),
      alignment: Alignment.center,
      child: Text(
        AppLocalizations.of(context).archivedBadge,
        style: AppTextStyle.caption(color: brewbook.palette.inkMuted).copyWith(
          fontWeight: FontWeight.w500,
        ),
      ),
    );
  }
}
