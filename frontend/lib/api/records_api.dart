/// 記録の API (FR-6 から FR-13) の呼び出し。
///
/// 経路の組み立てと、応答の型への変換だけを行い、状態は持たない。一覧はカーソル方式で、
/// 1 ページの件数は既定の 50 件とする (PRD の性能)。
library;

import 'api_client.dart';
import 'models.dart';
import 'record_inputs.dart';

/// サジェスト (FR-13) の対象の項目名。値は API の経路の名前と同じにする。
abstract final class SuggestionFields {
  /// 商品の生産者。
  static const String producer = 'producer';

  /// 商品の生産国。
  static const String origin = 'origin';

  /// 商品の地域。
  static const String region = 'region';

  /// 商品の精製方法。
  static const String process = 'process';

  /// 商品の品種。
  static const String variety = 'variety';

  /// 購入の焙煎度。
  static const String roast = 'roast';

  /// 抽出の抽出方法。
  static const String method = 'method';

  /// 抽出の挽き目。
  static const String grindSetting = 'grind_setting';
}

/// 写真のアップロード用 URL の発行の応答 (FR-10)。
class PhotoUploadTarget {
  const PhotoUploadTarget({required this.url, required this.key});

  /// 署名付きの PUT の URL (R2 の S3 互換エンドポイント)。
  final String url;

  /// 発行されたオブジェクトキー。完了の通知でそのまま返す。
  final String key;
}

/// 記録の API を [ApiClient] で呼ぶ。
class RecordsApi {
  RecordsApi(this._api);

  final ApiClient _api;

  /// 一覧の 1 ページの件数 (API の既定と同じ。PRD の性能)。
  static const int pageSize = 50;

  /// 店の一覧を引く (FR-6)。
  Future<RecordPage<Shop>> shops({String? cursor, bool includeArchived = false}) async {
    final json = await _api.getJson(
      _listPath('/shops', cursor: cursor, includeArchived: includeArchived),
    );
    return RecordPage<Shop>(
      items: _objects(json, 'shops').map(Shop.fromJson).toList(),
      nextCursor: _nextCursor(json),
    );
  }

  /// 店を 1 件引く (FR-6)。アーカイブ済みでも返る (FR-12)。
  Future<Shop> shop(String id) async => Shop.fromJson(await _api.getJson('/shops/$id'));

  /// 店を登録する (FR-6)。
  Future<Shop> createShop(ShopInput input) async {
    return Shop.fromJson(await _api.postJson('/shops', input.toJson()));
  }

  /// 店を更新する (FR-6)。
  Future<Shop> updateShop(String id, ShopInput input) async {
    return Shop.fromJson(await _api.patchJson('/shops/$id', input.toJson()));
  }

  /// 店をアーカイブする、またはアーカイブ解除する (FR-12)。
  Future<Shop> setShopArchived(String id, bool archived) async {
    return Shop.fromJson(await _api.postJson(_archivePath('/shops', id, archived)));
  }

  /// 商品の一覧を引く (FR-7、FR-8)。
  Future<RecordPage<Product>> products({String? cursor, bool includeArchived = false}) async {
    final json = await _api.getJson(
      _listPath('/products', cursor: cursor, includeArchived: includeArchived),
    );
    return RecordPage<Product>(
      items: _objects(json, 'products').map(Product.fromJson).toList(),
      nextCursor: _nextCursor(json),
    );
  }

  /// 商品を 1 件引く (FR-7)。アーカイブ済みでも返る (FR-12)。
  Future<Product> product(String id) async {
    return Product.fromJson(await _api.getJson('/products/$id'));
  }

  /// 商品を登録する (FR-7、FR-8)。
  Future<Product> createProduct(ProductInput input) async {
    return Product.fromJson(await _api.postJson('/products', input.toJson()));
  }

  /// 商品を更新する (FR-7、FR-8)。タグは入力した配列で置き換える (FR-8)。
  Future<Product> updateProduct(String id, ProductInput input) async {
    return Product.fromJson(await _api.patchJson('/products/$id', input.toJson()));
  }

  /// 商品をアーカイブする、またはアーカイブ解除する (FR-12)。
  Future<Product> setProductArchived(String id, bool archived) async {
    return Product.fromJson(await _api.postJson(_archivePath('/products', id, archived)));
  }

  /// 購入の一覧を引く (FR-9)。
  Future<RecordPage<Purchase>> purchases({String? cursor, bool includeArchived = false}) async {
    final json = await _api.getJson(
      _listPath('/purchases', cursor: cursor, includeArchived: includeArchived),
    );
    return RecordPage<Purchase>(
      items: _objects(json, 'purchases').map(Purchase.fromJson).toList(),
      nextCursor: _nextCursor(json),
    );
  }

  /// 購入を 1 件引く (FR-9)。アーカイブ済みでも返る (FR-12)。
  Future<Purchase> purchase(String id) async {
    return Purchase.fromJson(await _api.getJson('/purchases/$id'));
  }

  /// 購入を登録する (FR-9)。
  Future<Purchase> createPurchase(PurchaseInput input) async {
    return Purchase.fromJson(await _api.postJson('/purchases', input.toJson()));
  }

  /// 購入を更新する (FR-9)。
  Future<Purchase> updatePurchase(String id, PurchaseInput input) async {
    return Purchase.fromJson(await _api.patchJson('/purchases/$id', input.toJson()));
  }

  /// 購入をアーカイブする、またはアーカイブ解除する (FR-12)。
  Future<Purchase> setPurchaseArchived(String id, bool archived) async {
    return Purchase.fromJson(await _api.postJson(_archivePath('/purchases', id, archived)));
  }

  /// 写真のアップロード用 URL を発行する (FR-10)。変換後のサイズを申告する。
  Future<PhotoUploadTarget> requestPhotoUploadUrl(String purchaseId, int size) async {
    final json = await _api.postJson('/purchases/$purchaseId/photo/upload-url', <String, Object?>{
      'size': size,
    });
    final url = json['url'];
    final key = json['key'];
    if (url is! String || key is! String) {
      throw const FormatException('the upload url response must have a url and a key');
    }
    return PhotoUploadTarget(url: url, key: key);
  }

  /// 写真のアップロードの完了を通知する (FR-10)。紐づいた購入を返す。
  Future<Purchase> completePhoto(String purchaseId, {required String key, required int size}) async {
    final json = await _api.postJson('/purchases/$purchaseId/photo', <String, Object?>{
      'key': key,
      'size': size,
    });
    return Purchase.fromJson(json);
  }

  /// 写真を削除する (FR-10)。紐づけを外した購入を返す。
  Future<Purchase> deletePhoto(String purchaseId) async {
    return Purchase.fromJson(await _api.deleteJson('/purchases/$purchaseId/photo'));
  }

  /// 写真の取得の URL (FR-10)。写真は Backend が認証付きで返すため、この URL を表示に使う。
  /// 他の呼び出しと同じく、`ApiClient` の `basePath` を基準にする。
  Uri photoUrl(String purchaseId) =>
      Uri.base.resolve('${_api.basePath}/purchases/$purchaseId/photo');

  /// 抽出の一覧を引く (FR-11)。
  Future<RecordPage<Brew>> brews({String? cursor, bool includeArchived = false}) async {
    final json = await _api.getJson(
      _listPath('/brews', cursor: cursor, includeArchived: includeArchived),
    );
    return RecordPage<Brew>(
      items: _objects(json, 'brews').map(Brew.fromJson).toList(),
      nextCursor: _nextCursor(json),
    );
  }

  /// 抽出を 1 件引く (FR-11)。アーカイブ済みでも返る (FR-12)。
  Future<Brew> brew(String id) async => Brew.fromJson(await _api.getJson('/brews/$id'));

  /// 抽出を登録する (FR-11)。
  Future<Brew> createBrew(BrewInput input) async {
    return Brew.fromJson(await _api.postJson('/brews', input.toJson()));
  }

  /// 抽出を更新する (FR-11)。
  Future<Brew> updateBrew(String id, BrewInput input) async {
    return Brew.fromJson(await _api.patchJson('/brews/$id', input.toJson()));
  }

  /// 抽出をアーカイブする、またはアーカイブ解除する (FR-12)。
  Future<Brew> setBrewArchived(String id, bool archived) async {
    return Brew.fromJson(await _api.postJson(_archivePath('/brews', id, archived)));
  }

  /// 自由記述の項目の過去の入力値の候補を引く (FR-13)。
  ///
  /// 入力中の文字列で前方一致する候補を、API が最大 20 件返す。候補に無い値も入力できる。
  Future<List<String>> suggestions(String field, String query) async {
    final json = await _api.getJson(
      '/suggestions/$field?q=${Uri.encodeQueryComponent(query)}',
    );
    final values = json['values'];
    if (values is! List<Object?>) {
      throw const FormatException('the suggestions response must have an array of values');
    }
    return values.whereType<String>().toList();
  }

  /// 一覧の経路に、件数とカーソルとアーカイブの指定を付ける。
  static String _listPath(String path, {String? cursor, required bool includeArchived}) {
    final params = <String, String>{'limit': '$pageSize'};
    if (includeArchived) {
      params['include_archived'] = 'true';
    }
    if (cursor != null) {
      params['cursor'] = cursor;
    }
    return Uri(path: path, queryParameters: params).toString();
  }

  /// アーカイブと解除の経路 (FR-12)。
  static String _archivePath(String path, String id, bool archived) {
    return '$path/$id/${archived ? 'archive' : 'unarchive'}';
  }

  /// 応答の配列の項目を読む。配列でなければ応答の形式の違反にする。
  static List<Map<String, Object?>> _objects(Map<String, Object?> json, String key) {
    final value = json[key];
    if (value is List<Object?>) {
      return value.whereType<Map<String, Object?>>().toList();
    }
    throw FormatException('the $key field must be an array but was $value');
  }

  /// 応答の続きのカーソルを読む。文字列でも null でもない値は応答の形式の違反にする
  /// (他の読み取りと同じ扱い。形式の違反で一覧を静かに打ち切らない)。
  static String? _nextCursor(Map<String, Object?> json) {
    final value = json['next_cursor'];
    if (value == null) {
      return null;
    }
    if (value is String) {
      return value;
    }
    throw FormatException('the next_cursor field must be a string or null but was $value');
  }
}
