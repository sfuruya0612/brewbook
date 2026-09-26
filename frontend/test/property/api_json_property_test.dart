// API の入力の形と応答の型の対応の PBT (issue 0027、ADR-0013)。
//
// 画面が作る入力 (record_inputs.dart) の JSON に、応答に必要な id と日時と入れ子の
// オブジェクト (購入の product と shop、抽出の purchase) を足し、応答の型
// (models.dart) で読み戻して、重なる項目の値が一致することを検証する。
// null を生成するのは、店の住所、商品の Producer、Origin、Region、Process、Variety、
// 購入の Roast、価格、重量、購入の店の有無である (抽出の項目は非 null のみ)。
// 反例が出たときは、失敗の出力にある seed の値を forAll の seed に指定して再現する。
import 'package:brew_book/api/models.dart';
import 'package:brew_book/api/record_inputs.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:kiri_check/kiri_check.dart';

const String _timestamp = '2026-09-26T00:00:00.000Z';
const String _day = '2026-09-26';

/// 任意の文字列 (null を含む)。
Arbitrary<String?> nullableString() => oneOf([
      string(maxLength: 10),
      constant(null),
    ]).map((value) => value as String?);

/// 任意の整数 (null を含む)。
Arbitrary<int?> nullableInt() => oneOf([
      integer(min: 0, max: 1000000),
      constant(null),
    ]).map((value) => value as int?);

/// 応答だけが持つ項目 (id と日時) を足す。
Map<String, Object?> withResponseFields(Map<String, Object?> json) =>
    <String, Object?>{
      ...json,
      'id': 'id-1',
      'created_at': _timestamp,
      'updated_at': _timestamp,
      'archived_at': null,
    };

Map<String, Object?> productJson() => <String, Object?>{
      'id': 'product-1',
      'name': 'Product',
      'producer': null,
      'origin': null,
      'region': null,
      'process': null,
      'variety': null,
      'flavor_notes': <String>[],
      'created_at': _timestamp,
      'updated_at': _timestamp,
      'archived_at': null,
    };

Map<String, Object?> shopJson() => <String, Object?>{
      'id': 'shop-1',
      'name': 'Shop',
      'address': null,
      'created_at': _timestamp,
      'updated_at': _timestamp,
      'archived_at': null,
    };

void main() {
  property('店の入力の形は応答の型から読み戻せる', () {
    forAll(
      combine2(string(maxLength: 20), nullableString()),
      (input) {
        final (name, address) = input;
        final json = ShopInput(name: name, address: address).toJson();

        final shop = Shop.fromJson(withResponseFields(json));

        expect(shop.name, name);
        expect(shop.address, address);
      },
    );
  });

  property('商品の入力の形は応答の型から読み戻せる', () {
    forAll(
      combine7(
        string(maxLength: 20),
        nullableString(),
        nullableString(),
        nullableString(),
        nullableString(),
        nullableString(),
        list(string(maxLength: 10), maxLength: 3),
      ),
      (input) {
        final (name, producer, origin, region, process, variety, flavorNotes) =
            input;
        final json = ProductInput(
          name: name,
          producer: producer,
          origin: origin,
          region: region,
          process: process,
          variety: variety,
          flavorNotes: flavorNotes,
        ).toJson();

        final product = Product.fromJson(withResponseFields(json));

        expect(product.name, name);
        expect(product.producer, producer);
        expect(product.origin, origin);
        expect(product.region, region);
        expect(product.process, process);
        expect(product.variety, variety);
        expect(product.flavorNotes, flavorNotes);
      },
    );
  });

  property('購入の入力の形は応答の型から読み戻せる', () {
    forAll(
      combine4(
        nullableString(),
        nullableInt(),
        nullableInt(),
        boolean(),
      ),
      (input) {
        final (roast, priceAmount, weightGrams, hasShop) = input;
        final json = PurchaseInput(
          productId: 'product-1',
          shopId: hasShop ? 'shop-1' : null,
          purchasedOn: _day,
          roast: roast,
          roastDate: _day,
          priceAmount: priceAmount,
          priceCurrency: priceAmount == null ? null : 'JPY',
          weightGrams: weightGrams,
        ).toJson();

        final purchase = Purchase.fromJson(<String, Object?>{
          ...withResponseFields(json),
          'product': productJson(),
          'shop': hasShop ? shopJson() : null,
        });

        expect(purchase.productId, 'product-1');
        expect(purchase.shopId, hasShop ? 'shop-1' : null);
        expect(purchase.purchasedOn, _day);
        expect(purchase.roast, roast);
        expect(purchase.roastDate, _day);
        expect(purchase.priceAmount, priceAmount);
        expect(purchase.priceCurrency, priceAmount == null ? null : 'JPY');
        expect(purchase.weightGrams, weightGrams);
      },
    );
  });

  property('抽出の入力の形は応答の型から読み戻せる', () {
    forAll(
      combine5(
        integer(min: 0, max: 10000),
        integer(min: 0, max: 10000),
        integer(min: 0, max: 10000),
        string(maxLength: 10),
        integer(min: 1, max: 5),
      ),
      (input) {
        final (doseTenths, waterTenths, tempTenths, method, rating) = input;
        final json = BrewInput(
          purchaseId: 'purchase-1',
          brewedAt: _timestamp,
          doseGrams: doseTenths / 10,
          waterGrams: waterTenths / 10,
          waterTempC: tempTenths / 10,
          brewTimeSeconds: 120,
          method: method,
          grindSetting: 'medium',
          rating: rating,
          notes: 'notes',
        ).toJson();

        final brew = Brew.fromJson(<String, Object?>{
          ...withResponseFields(json),
          'purchase': <String, Object?>{
            ...withResponseFields(<String, Object?>{
              'product_id': 'product-1',
              'shop_id': null,
              'purchased_on': _day,
              'roast': null,
              'roast_date': null,
              'price_amount': null,
              'price_currency': null,
              'weight_grams': null,
            }),
            'product': productJson(),
            'shop': null,
          },
        });

        expect(brew.purchaseId, 'purchase-1');
        expect(brew.brewedAt, _timestamp);
        expect(brew.doseGrams, doseTenths / 10);
        expect(brew.waterGrams, waterTenths / 10);
        expect(brew.waterTempC, tempTenths / 10);
        expect(brew.brewTimeSeconds, 120);
        expect(brew.method, method);
        expect(brew.grindSetting, 'medium');
        expect(brew.rating, rating);
        expect(brew.notes, 'notes');
      },
    );
  });
}
