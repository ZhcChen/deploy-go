// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'external_api_key_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ExternalApiKeyListResponse extends ExternalApiKeyListResponse {
  @override
  final BuiltList<ExternalApiKeySummary> items;
  @override
  final String? nextCursor;

  factory _$ExternalApiKeyListResponse([
    void Function(ExternalApiKeyListResponseBuilder)? updates,
  ]) => (ExternalApiKeyListResponseBuilder()..update(updates))._build();

  _$ExternalApiKeyListResponse._({required this.items, this.nextCursor})
    : super._();
  @override
  ExternalApiKeyListResponse rebuild(
    void Function(ExternalApiKeyListResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ExternalApiKeyListResponseBuilder toBuilder() =>
      ExternalApiKeyListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ExternalApiKeyListResponse &&
        items == other.items &&
        nextCursor == other.nextCursor;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, items.hashCode);
    _$hash = $jc(_$hash, nextCursor.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ExternalApiKeyListResponse')
          ..add('items', items)
          ..add('nextCursor', nextCursor))
        .toString();
  }
}

class ExternalApiKeyListResponseBuilder
    implements
        Builder<ExternalApiKeyListResponse, ExternalApiKeyListResponseBuilder> {
  _$ExternalApiKeyListResponse? _$v;

  ListBuilder<ExternalApiKeySummary>? _items;
  ListBuilder<ExternalApiKeySummary> get items =>
      _$this._items ??= ListBuilder<ExternalApiKeySummary>();
  set items(ListBuilder<ExternalApiKeySummary>? items) => _$this._items = items;

  String? _nextCursor;
  String? get nextCursor => _$this._nextCursor;
  set nextCursor(String? nextCursor) => _$this._nextCursor = nextCursor;

  ExternalApiKeyListResponseBuilder() {
    ExternalApiKeyListResponse._defaults(this);
  }

  ExternalApiKeyListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _nextCursor = $v.nextCursor;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ExternalApiKeyListResponse other) {
    _$v = other as _$ExternalApiKeyListResponse;
  }

  @override
  void update(void Function(ExternalApiKeyListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ExternalApiKeyListResponse build() => _build();

  _$ExternalApiKeyListResponse _build() {
    _$ExternalApiKeyListResponse _$result;
    try {
      _$result =
          _$v ??
          _$ExternalApiKeyListResponse._(
            items: items.build(),
            nextCursor: nextCursor,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'items';
        items.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ExternalApiKeyListResponse',
          _$failedField,
          e.toString(),
        );
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
