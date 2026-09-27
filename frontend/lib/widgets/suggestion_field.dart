import 'package:flutter/material.dart';

import '../api/records_api.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import 'app_field.dart';

/// 自由記述の項目の入力欄 (FR-13)。
///
/// 入力中にサジェスト API を呼び、過去の入力値を候補として最大 20 件表示する。候補の
/// 選択は任意で、候補に無い値もそのまま入力できる。候補を引けなくても入力は妨げない。
/// 一致した先頭部分は crema-ink の 600 で示す (docs/design/components/Field)。
class SuggestionField extends StatefulWidget {
  const SuggestionField({
    super.key,
    required this.records,
    required this.controller,
    required this.field,
    required this.label,
    this.enabled = true,
    this.errorText,
    this.hintText,
  });

  /// サジェスト API (FR-13) の呼び出し。
  final RecordsApi records;

  final TextEditingController controller;

  /// サジェストの項目名 ([SuggestionFields])。
  final String field;
  final String label;
  final bool enabled;
  final String? errorText;
  final String? hintText;

  @override
  State<SuggestionField> createState() => _SuggestionFieldState();
}

class _SuggestionFieldState extends State<SuggestionField> {
  final FocusNode _focus = FocusNode();

  /// 直近に送った要求の番号。遅れて届いた応答を捨てるために使う。
  int _sequence = 0;

  @override
  void dispose() {
    _focus.dispose();
    super.dispose();
  }

  /// 入力中の文字列に対する候補を引く。
  Future<Iterable<String>> _options(TextEditingValue value) async {
    final sequence = ++_sequence;
    try {
      final values = await widget.records.suggestions(widget.field, value.text);
      // 入力が進んだ後に届いた応答は捨てる。
      return sequence == _sequence ? values : const <String>[];
    } catch (error) {
      // 候補は入力の補助であり、引けなくても入力は続けられる (FR-13)。
      return const <String>[];
    }
  }

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final bool hasError = widget.errorText != null;
    return RawAutocomplete<String>(
      textEditingController: widget.controller,
      focusNode: _focus,
      optionsBuilder: _options,
      optionsViewOpenDirection: OptionsViewOpenDirection.down,
      displayStringForOption: (option) => option,
      onSelected: (value) => widget.controller.value = TextEditingValue(
        text: value,
        selection: TextSelection.collapsed(offset: value.length),
      ),
      fieldViewBuilder: (context, controller, focusNode, onFieldSubmitted) => AppField(
        label: widget.label,
        errorText: widget.errorText,
        child: TextField(
          controller: controller,
          focusNode: focusNode,
          enabled: widget.enabled,
          style: AppTextStyle.body(color: brewbook.palette.ink),
          decoration: InputDecoration(
            hintText: widget.hintText,
            enabledBorder: hasError
                ? OutlineInputBorder(
                    borderRadius: AppRadius.smAll,
                    borderSide: BorderSide(color: brewbook.palette.signal),
                  )
                : null,
            focusedBorder: hasError
                ? OutlineInputBorder(
                    borderRadius: AppRadius.smAll,
                    borderSide: BorderSide(color: brewbook.palette.signal),
                  )
                : null,
          ),
          onSubmitted: (_) => onFieldSubmitted(),
        ),
      ),
      optionsViewBuilder: (context, onSelected, options) => Align(
        alignment: Alignment.topLeft,
        child: Material(
          color: brewbook.palette.paperRaised,
          elevation: 3,
          shape: RoundedRectangleBorder(
            borderRadius: const BorderRadius.vertical(bottom: Radius.circular(AppRadius.sm)),
            side: BorderSide(color: brewbook.lineStrong),
          ),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 240, maxWidth: 360),
            child: ListView.separated(
              shrinkWrap: true,
              padding: EdgeInsets.zero,
              itemCount: options.length,
              separatorBuilder: (context, index) => Divider(color: brewbook.line, height: 1),
              itemBuilder: (context, index) {
                final option = options.elementAt(index);
                return InkWell(
                  onTap: () => onSelected(option),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: AppSpacing.x3,
                      vertical: 10,
                    ),
                    child: _highlight(
                      context,
                      option,
                      widget.controller.text,
                    ),
                  ),
                );
              },
            ),
          ),
        ),
      ),
    );
  }

  /// 一致した先頭部分を crema-ink の 600 で示す。
  Widget _highlight(BuildContext context, String option, String input) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final String query = input.trim().toLowerCase();
    final TextStyle base = AppTextStyle.body(color: brewbook.palette.ink);
    if (query.isEmpty || !option.toLowerCase().startsWith(query)) {
      return Text(option, style: base);
    }
    return Text.rich(
      TextSpan(
        children: <InlineSpan>[
          TextSpan(
            text: option.substring(0, query.length),
            style: base.copyWith(color: brewbook.cremaInk, fontWeight: FontWeight.w600),
          ),
          TextSpan(text: option.substring(query.length), style: base),
        ],
      ),
    );
  }
}
