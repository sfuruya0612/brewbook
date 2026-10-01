// 通貨の選択肢の PBT (issue 0035、ADR-0013)。
//
// 生成する入力は任意の文字列と null。マスタのコードは一様な文字列生成では引けないため、
// 代表する定数 (マスタの全 30 種とマスタに無いコード) を足す (values_property_test と同じ方針)。
// 反例が出たときは、失敗の出力にある seed の値を forAll の seed に指定して再現する。
import 'package:brew_book/records/currencies.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:kiri_check/kiri_check.dart';

/// 任意の通貨コード。マスタの全 30 種、マスタに無い代表、任意の文字列、null を生成する。
Arbitrary<String?> arbitraryCurrencyCode() => oneOf([
      for (final String code in currencyCodes) constant(code),
      constant('ETB'),
      string(maxLength: 10),
      constant(null),
    ]).map((value) => value as String?);

void main() {
  property('選択肢はマスタの 30 種を保ち、入力のコードを高々 1 つ足し、コードの昇順になる', () {
    forAll(arbitraryCurrencyCode(), (current) {
      final List<String> options = currencyOptions(current);
      final bool isMaster = current != null && currencyCodes.contains(current);

      // マスタの 30 種は常に含む。
      expect(options, containsAll(currencyCodes));
      if (current == null || isMaster) {
        // マスタにあるコード (と null) のときは、マスタの選択肢だけにする。
        expect(options, currencyCodes);
      } else {
        // マスタに無いコードのときだけ足す。
        expect(options.length, currencyCodes.length + 1);
        expect(options, contains(current));
      }
      // 入力のコードは高々 1 つで、コードの昇順になる。
      expect(options.where((String code) => code == current).length, lessThanOrEqualTo(1));
      expect(options, orderedEquals(<String>[...options]..sort()));
    });
  });
}
