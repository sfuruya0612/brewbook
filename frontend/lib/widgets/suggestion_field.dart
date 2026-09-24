import 'package:flutter/material.dart';

import '../api/records_api.dart';

/// 自由記述の項目の入力欄 (FR-13)。
///
/// 入力中にサジェスト API を呼び、過去の入力値を候補として最大 20 件表示する。候補の選択は
/// 任意で、候補に無い値もそのまま入力できる。候補を引けなくても入力は妨げない。
class SuggestionField extends StatefulWidget {
  const SuggestionField({
    super.key,
    required this.records,
    required this.controller,
    required this.field,
    required this.label,
    this.enabled = true,
    this.errorText,
  });

  /// サジェスト API (FR-13) の呼び出し。
  final RecordsApi records;

  final TextEditingController controller;

  /// サジェストの項目名 ([SuggestionFields])。
  final String field;
  final String label;
  final bool enabled;
  final String? errorText;

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
      fieldViewBuilder: (context, controller, focusNode, onFieldSubmitted) => TextField(
        controller: controller,
        focusNode: focusNode,
        enabled: widget.enabled,
        decoration: InputDecoration(
          labelText: widget.label,
          errorText: widget.errorText,
          suffixIcon: const Icon(Icons.arrow_drop_down),
        ),
        onSubmitted: (_) => onFieldSubmitted(),
      ),
      optionsViewBuilder: (context, onSelected, options) => Align(
        alignment: Alignment.topLeft,
        child: Material(
          elevation: 4,
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 240, maxWidth: 360),
            child: ListView.builder(
              shrinkWrap: true,
              itemCount: options.length,
              itemBuilder: (context, index) {
                final option = options.elementAt(index);
                return ListTile(
                  dense: true,
                  title: Text(option),
                  onTap: () => onSelected(option),
                );
              },
            ),
          ),
        ),
      ),
    );
  }
}
