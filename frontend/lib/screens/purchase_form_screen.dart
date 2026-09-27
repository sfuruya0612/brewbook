import 'package:flutter/material.dart';

import '../api/models.dart';
import '../api/record_inputs.dart';
import '../api/records_api.dart';
import '../l10n/app_localizations.dart';
import '../photo/image_converter.dart';
import '../photo/photo_picker.dart';
import '../records/record_services.dart';
import '../records/values.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/app_field.dart';
import '../widgets/app_form.dart';
import '../widgets/day_time_fields.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';
import '../widgets/picker_tile.dart';
import '../widgets/record_picker.dart';
import '../widgets/suggestion_field.dart';

/// ISO 4217 の通貨コードの形 (英大文字 3 文字)。
final RegExp _currencyPattern = RegExp(r'^[A-Z]{3}$');

/// 通貨コードの既定値 (FR-9)。
const String defaultCurrency = 'JPY';

/// 購入の登録と編集の画面 (FR-9、FR-10)。
///
/// 商品は必須で、店は省略できる。購入日は端末のタイムゾーンでの当日を既定値にする (FR-9)。
/// 写真は選択した時点では変換だけ行い、購入を保存した後にアップロードする (FR-10、ADR-0003)。
class PurchaseFormScreen extends StatefulWidget {
  const PurchaseFormScreen({
    super.key,
    required this.services,
    this.id,
    this.embedded = false,
    this.onClose,
    this.onSaved,
  });

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  /// 編集する購入の ID。新規の登録のときは null。
  final String? id;

  /// 幅 840 px 以上の 2 段組の右の面に出すか。
  final bool embedded;

  /// 閉じる動き。無いときは前の画面へ戻る。
  final VoidCallback? onClose;

  /// 保存できたときの動き。無いときは前の画面へ戻る。
  final VoidCallback? onSaved;

  @override
  State<PurchaseFormScreen> createState() => _PurchaseFormScreenState();
}

class _PurchaseFormScreenState extends State<PurchaseFormScreen> {
  final TextEditingController _purchasedOn = TextEditingController();
  final TextEditingController _roast = TextEditingController();
  final TextEditingController _roastDate = TextEditingController();
  final TextEditingController _price = TextEditingController();
  final TextEditingController _currency = TextEditingController(text: defaultCurrency);
  final TextEditingController _weight = TextEditingController();

  Product? _product;
  Shop? _shop;

  /// アップロード済みの写真のキー。編集で読み込む。
  String? _photoKey;

  /// 選択された写真 (まだアップロードしていない)。
  PickedPhoto? _picked;

  /// 選択された写真を変換したもの (サイズの申告に使う。FR-10)。
  ConvertedImage? _converted;

  /// アップロード済みの写真を消す操作をしたか。
  bool _removePhoto = false;

  /// この画面で登録した購入の ID。
  ///
  /// 写真のアップロードに失敗して保存をやり直すとき、同じ購入を更新するために使う
  /// (再試行で同じ購入が 2 つできないようにする)。
  String? _createdId;

  String? _productError;
  String? _purchasedOnError;
  String? _roastDateError;
  String? _priceError;
  String? _currencyError;
  String? _weightError;
  String? _errorMessage;
  String? _loadError;

  /// 検証の誤りを上のバナーで示すか。
  bool _showValidationBanner = false;
  bool _busy = false;
  bool _loading = false;

  @override
  void initState() {
    super.initState();
    _purchasedOn.text = formatDay(today());
    final id = widget.id;
    if (id != null) {
      _load(id);
    }
  }

  @override
  void dispose() {
    _purchasedOn.dispose();
    _roast.dispose();
    _roastDate.dispose();
    _price.dispose();
    _currency.dispose();
    _weight.dispose();
    super.dispose();
  }

  /// 編集のために現在の値を読み込む。
  Future<void> _load(String id) async {
    // 再試行で回復できるよう、前回の失敗の表示を消してから読み直す。
    setState(() {
      _loading = true;
      _loadError = null;
    });
    try {
      final purchase = await widget.services.records.purchase(id);
      if (!mounted) {
        return;
      }
      setState(() {
        _product = purchase.product;
        _shop = purchase.shop;
        _purchasedOn.text = purchase.purchasedOn;
        _roast.text = purchase.roast ?? '';
        _roastDate.text = purchase.roastDate ?? '';
        _price.text = purchase.priceAmount?.toString() ?? '';
        _currency.text = purchase.priceCurrency ?? defaultCurrency;
        _weight.text = purchase.weightGrams?.toString() ?? '';
        _photoKey = purchase.photoKey;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _loadError = messageForError(error, AppLocalizations.of(context)));
    } finally {
      if (mounted) {
        setState(() => _loading = false);
      }
    }
  }

  /// 商品を選ばせる (必須。FR-9)。
  Future<void> _pickProduct() async {
    final l10n = AppLocalizations.of(context);
    final choice = await showRecordPicker<Product>(
      context: context,
      title: l10n.selectProductTitle,
      services: widget.services,
      load: ({cursor, required includeArchived}) =>
          widget.services.records.products(cursor: cursor, includeArchived: false),
      titleOf: (context, product) => product.name,
      subtitleOf: (context, product) {
        final producer = product.producer;
        return producer == null || producer.isEmpty ? null : producer;
      },
    );
    if (choice == null || !mounted) {
      return;
    }
    setState(() {
      _product = choice.value;
      _productError = null;
    });
  }

  /// 店を選ばせる (任意。FR-9)。選ばない選択肢も置く。
  Future<void> _pickShop() async {
    final l10n = AppLocalizations.of(context);
    final choice = await showRecordPicker<Shop>(
      context: context,
      title: l10n.selectShopTitle,
      services: widget.services,
      load: ({cursor, required includeArchived}) =>
          widget.services.records.shops(cursor: cursor, includeArchived: false),
      titleOf: (context, shop) => shop.name,
      subtitleOf: (context, shop) {
        final address = shop.address;
        return address == null || address.isEmpty ? null : address;
      },
      clearLabel: l10n.shopNoneLabel,
    );
    if (choice == null || !mounted) {
      return;
    }
    setState(() => _shop = choice.value);
  }

  /// 写真を選び、JPEG に変換して長辺を縮める (FR-10、ADR-0003)。
  ///
  /// アップロードは購入を保存した後に行う (URL の発行に購入の ID が要るため)。
  Future<void> _pickPhoto() async {
    final l10n = AppLocalizations.of(context);
    setState(() => _errorMessage = null);
    try {
      final photo = await widget.services.picker.pickPhoto();
      if (photo == null) {
        return;
      }
      final converted = await widget.services.converter.convertJpeg(photo.bytes);
      if (!mounted) {
        return;
      }
      setState(() {
        _picked = photo;
        _converted = converted;
        _removePhoto = false;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _errorMessage = messageForError(error, l10n));
    }
  }

  /// 写真の選択を取り消す、またはアップロード済みの写真を消す操作を記録する (FR-10)。
  ///
  /// アップロード済みの写真の削除は、保存のときに API を呼ぶ。
  void _deletePhoto() {
    setState(() {
      _picked = null;
      _converted = null;
      if (_photoKey != null) {
        _removePhoto = true;
      }
    });
  }

  /// 入力した値で登録または更新し、写真があればアップロードする。
  Future<void> _save() async {
    final l10n = AppLocalizations.of(context);
    final product = _product;
    final purchasedOn = parseDay(_purchasedOn.text);
    final roastDateText = _roastDate.text.trim();
    final roastDate = roastDateText.isEmpty ? null : parseDay(roastDateText);
    final priceText = _price.text.trim();
    final price = priceText.isEmpty ? null : parseCount(priceText);
    final currency = _currency.text.trim().toUpperCase();
    final weightText = _weight.text.trim();
    final weight = weightText.isEmpty ? null : parseCount(weightText);

    final productError = product == null ? l10n.validationProduct : null;
    final purchasedOnError = purchasedOn == null ? l10n.validationDay : null;
    final roastDateError =
        roastDateText.isNotEmpty && roastDate == null ? l10n.validationDay : null;
    final priceError = priceText.isNotEmpty && price == null ? l10n.validationNumber : null;
    final currencyError = price != null && !_currencyPattern.hasMatch(currency)
        ? l10n.validationCurrency
        : null;
    final weightError =
        weightText.isNotEmpty && weight == null ? l10n.validationNumber : null;
    setState(() {
      _productError = productError;
      _purchasedOnError = purchasedOnError;
      _roastDateError = roastDateError;
      _priceError = priceError;
      _currencyError = currencyError;
      _weightError = weightError;
      _errorMessage = null;
      _showValidationBanner = productError != null ||
          purchasedOnError != null ||
          roastDateError != null ||
          priceError != null ||
          currencyError != null ||
          weightError != null;
    });
    if (_showValidationBanner) {
      return;
    }

    final input = PurchaseInput(
      productId: product!.id,
      shopId: _shop?.id,
      purchasedOn: formatDay(purchasedOn!),
      roast: _optionalText(_roast),
      roastDate: roastDate == null ? null : formatDay(roastDate),
      priceAmount: price,
      priceCurrency: price == null ? null : currency,
      weightGrams: weight,
    );
    setState(() => _busy = true);
    try {
      // 登録の後に写真のアップロードで失敗した場合のやり直しでは、同じ購入を更新する。
      final id = widget.id ?? _createdId;
      // 写真の削除は購入の更新の前に行う (差し替えでは、この後に新しい写真を紐づける。FR-10)。
      if (id != null && _removePhoto && _photoKey != null) {
        await widget.services.records.deletePhoto(id);
        // 削除に成功したら状態を更新する。購入の更新に失敗してやり直しても、
        // 2 回目の削除が 404 にならないようにするためである。
        _photoKey = null;
        _removePhoto = false;
        // 購入の更新が失敗しても、写真が消えたことを一覧と詳細に伝える。
        widget.services.markRecordsChanged();
      }
      final purchase = id == null
          ? await widget.services.records.createPurchase(input)
          : await widget.services.records.updatePurchase(id, input);
      // 写真のアップロードに失敗しても、やり直しで同じ購入を更新できるようにする。
      _createdId = purchase.id;
      // 変換した写真を、購入の ID ができてからアップロードする (FR-10)。
      final converted = _converted;
      if (converted != null) {
        await widget.services.uploader.upload(purchaseId: purchase.id, image: converted);
      }
      widget.services.markRecordsChanged();
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.savedMessage)),
      );
      if (widget.embedded) {
        widget.onSaved?.call();
      } else {
        Navigator.of(context).pop();
      }
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _errorMessage = messageForError(error, l10n));
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// 空の入力を API の null にする。
  static String? _optionalText(TextEditingController controller) {
    final value = controller.text.trim();
    return value.isEmpty ? null : value;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(
        leading: CloseButton(
          onPressed: widget.onClose ?? () => Navigator.of(context).maybePop(),
        ),
        title: Text(widget.id == null ? l10n.purchaseNewTitle : l10n.purchaseEditTitle),
        actions: <Widget>[
          TextButton(
            onPressed: _busy ? null : _save,
            child: Text(l10n.saveButton),
          ),
          const SizedBox(width: AppSpacing.x2),
        ],
      ),
      body: _body(context, l10n),
    );
  }

  Widget _body(BuildContext context, AppLocalizations l10n) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    if (_loading) {
      return Center(
        child: Text(l10n.loading, style: AppTextStyle.body(color: brewbook.palette.inkMuted)),
      );
    }
    final loadError = _loadError;
    if (loadError != null) {
      return Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(AppSpacing.x6),
          child: ErrorBanner(message: loadError, onRetry: () => _load(widget.id!)),
        ),
      );
    }
    final records = widget.services.records;
    return AppForm(
      children: <Widget>[
        if (_showValidationBanner) ErrorBanner(message: l10n.errorValidation),
        PickerTile(
          label: l10n.productLabel,
          required: true,
          value: _product?.name,
          placeholder: l10n.selectProductTitle,
          errorText: _productError,
          onPressed: _busy ? null : _pickProduct,
        ),
        PickerTile(
          label: l10n.shopLabel,
          value: _shop?.name,
          placeholder: l10n.shopNoneLabel,
          onPressed: _busy ? null : _pickShop,
        ),
        DayField(
          controller: _purchasedOn,
          label: l10n.purchasedOnLabel,
          enabled: !_busy,
          errorText: _purchasedOnError,
        ),
        SuggestionField(
          records: records,
          controller: _roast,
          field: SuggestionFields.roast,
          label: l10n.roast,
          enabled: !_busy,
        ),
        DayField(
          controller: _roastDate,
          label: l10n.roastDate,
          enabled: !_busy,
          errorText: _roastDateError,
        ),
        AppFormRow(
          flex: const <int>[2, 1],
          children: <Widget>[
            AppTextField(
              controller: _price,
              label: l10n.priceLabel,
              enabled: !_busy,
              errorText: _priceError,
              keyboardType: TextInputType.number,
            ),
            AppTextField(
              controller: _currency,
              label: l10n.currencyLabel,
              enabled: !_busy,
              errorText: _currencyError,
            ),
          ],
        ),
        AppTextField(
          controller: _weight,
          label: l10n.weightLabel,
          enabled: !_busy,
          errorText: _weightError,
          unit: l10n.gramUnit,
          keyboardType: TextInputType.number,
        ),
        _photo(context, l10n),
        if (_errorMessage != null) ErrorBanner(message: _errorMessage!),
      ],
    );
  }

  /// 写真の選択、差し替え、削除の操作 (FR-10)。
  Widget _photo(BuildContext context, AppLocalizations l10n) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final picked = _picked;
    final converted = _converted;
    final hasUploaded = !_removePhoto && _photoKey != null;
    final hasPhoto = picked != null || hasUploaded;
    return AppField(
      label: l10n.photoLabel,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          ClipRRect(
            borderRadius: AppRadius.smAll,
            child: Container(
              width: 88,
              height: 88,
              color: brewbook.palette.paperSunken,
              alignment: Alignment.center,
              child: picked != null && converted != null
                  ? Image.memory(
                      converted.bytes,
                      key: const Key('purchase-photo'),
                      width: 88,
                      height: 88,
                      fit: BoxFit.cover,
                      // 復号できない写真でも画面は壊さない。
                      errorBuilder: (context, error, stackTrace) =>
                          Icon(Icons.photo_camera_outlined, size: 32, color: brewbook.inkFaint),
                    )
                  : hasUploaded
                  ? Image.network(
                      widget.services.records.photoUrl(widget.id!).toString(),
                      key: const Key('purchase-photo'),
                      width: 88,
                      height: 88,
                      fit: BoxFit.cover,
                      errorBuilder: (context, error, stackTrace) =>
                          Icon(Icons.photo_camera_outlined, size: 32, color: brewbook.inkFaint),
                    )
                  : Icon(Icons.photo_camera_outlined, size: 32, color: brewbook.inkFaint),
            ),
          ),
          const SizedBox(width: AppSpacing.x3),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                OutlinedButton(
                  onPressed: _busy ? null : _pickPhoto,
                  style: OutlinedButton.styleFrom(
                    minimumSize: const Size(0, 36),
                    padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x4),
                    textStyle: AppTextStyle.label(color: brewbook.palette.ink),
                  ),
                  child: Text(hasPhoto ? l10n.photoReplaceButton : l10n.photoSelectButton),
                ),
                if (hasPhoto) ...<Widget>[
                  const SizedBox(height: AppSpacing.x1),
                  TextButton(
                    onPressed: _busy ? null : _deletePhoto,
                    child: Text(l10n.photoDeleteButton),
                  ),
                ],
                const SizedBox(height: AppSpacing.x2),
                Text(
                  l10n.photoConvertNote,
                  style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
