// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'external_application_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ExternalApplicationListResponse
    extends ExternalApplicationListResponse {
  @override
  final BuiltList<ExternalApplicationSummary> items;

  factory _$ExternalApplicationListResponse([
    void Function(ExternalApplicationListResponseBuilder)? updates,
  ]) => (ExternalApplicationListResponseBuilder()..update(updates))._build();

  _$ExternalApplicationListResponse._({required this.items}) : super._();
  @override
  ExternalApplicationListResponse rebuild(
    void Function(ExternalApplicationListResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ExternalApplicationListResponseBuilder toBuilder() =>
      ExternalApplicationListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ExternalApplicationListResponse && items == other.items;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, items.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'ExternalApplicationListResponse',
    )..add('items', items)).toString();
  }
}

class ExternalApplicationListResponseBuilder
    implements
        Builder<
          ExternalApplicationListResponse,
          ExternalApplicationListResponseBuilder
        > {
  _$ExternalApplicationListResponse? _$v;

  ListBuilder<ExternalApplicationSummary>? _items;
  ListBuilder<ExternalApplicationSummary> get items =>
      _$this._items ??= ListBuilder<ExternalApplicationSummary>();
  set items(ListBuilder<ExternalApplicationSummary>? items) =>
      _$this._items = items;

  ExternalApplicationListResponseBuilder() {
    ExternalApplicationListResponse._defaults(this);
  }

  ExternalApplicationListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ExternalApplicationListResponse other) {
    _$v = other as _$ExternalApplicationListResponse;
  }

  @override
  void update(void Function(ExternalApplicationListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ExternalApplicationListResponse build() => _build();

  _$ExternalApplicationListResponse _build() {
    _$ExternalApplicationListResponse _$result;
    try {
      _$result =
          _$v ?? _$ExternalApplicationListResponse._(items: items.build());
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'items';
        items.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ExternalApplicationListResponse',
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
