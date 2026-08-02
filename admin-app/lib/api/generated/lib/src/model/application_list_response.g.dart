// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationListResponse extends ApplicationListResponse {
  @override
  final BuiltList<ApplicationResponse> items;
  @override
  final String? nextCursor;

  factory _$ApplicationListResponse(
          [void Function(ApplicationListResponseBuilder)? updates]) =>
      (ApplicationListResponseBuilder()..update(updates))._build();

  _$ApplicationListResponse._({required this.items, this.nextCursor})
      : super._();
  @override
  ApplicationListResponse rebuild(
          void Function(ApplicationListResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ApplicationListResponseBuilder toBuilder() =>
      ApplicationListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationListResponse &&
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
    return (newBuiltValueToStringHelper(r'ApplicationListResponse')
          ..add('items', items)
          ..add('nextCursor', nextCursor))
        .toString();
  }
}

class ApplicationListResponseBuilder
    implements
        Builder<ApplicationListResponse, ApplicationListResponseBuilder> {
  _$ApplicationListResponse? _$v;

  ListBuilder<ApplicationResponse>? _items;
  ListBuilder<ApplicationResponse> get items =>
      _$this._items ??= ListBuilder<ApplicationResponse>();
  set items(ListBuilder<ApplicationResponse>? items) => _$this._items = items;

  String? _nextCursor;
  String? get nextCursor => _$this._nextCursor;
  set nextCursor(String? nextCursor) => _$this._nextCursor = nextCursor;

  ApplicationListResponseBuilder() {
    ApplicationListResponse._defaults(this);
  }

  ApplicationListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _nextCursor = $v.nextCursor;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationListResponse other) {
    _$v = other as _$ApplicationListResponse;
  }

  @override
  void update(void Function(ApplicationListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationListResponse build() => _build();

  _$ApplicationListResponse _build() {
    _$ApplicationListResponse _$result;
    try {
      _$result = _$v ??
          _$ApplicationListResponse._(
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
            r'ApplicationListResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
