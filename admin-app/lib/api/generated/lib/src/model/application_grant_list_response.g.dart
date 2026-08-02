// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_grant_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationGrantListResponse extends ApplicationGrantListResponse {
  @override
  final BuiltList<ApplicationGrantResponse> items;
  @override
  final String? nextCursor;

  factory _$ApplicationGrantListResponse([
    void Function(ApplicationGrantListResponseBuilder)? updates,
  ]) => (ApplicationGrantListResponseBuilder()..update(updates))._build();

  _$ApplicationGrantListResponse._({required this.items, this.nextCursor})
    : super._();
  @override
  ApplicationGrantListResponse rebuild(
    void Function(ApplicationGrantListResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationGrantListResponseBuilder toBuilder() =>
      ApplicationGrantListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationGrantListResponse &&
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
    return (newBuiltValueToStringHelper(r'ApplicationGrantListResponse')
          ..add('items', items)
          ..add('nextCursor', nextCursor))
        .toString();
  }
}

class ApplicationGrantListResponseBuilder
    implements
        Builder<
          ApplicationGrantListResponse,
          ApplicationGrantListResponseBuilder
        > {
  _$ApplicationGrantListResponse? _$v;

  ListBuilder<ApplicationGrantResponse>? _items;
  ListBuilder<ApplicationGrantResponse> get items =>
      _$this._items ??= ListBuilder<ApplicationGrantResponse>();
  set items(ListBuilder<ApplicationGrantResponse>? items) =>
      _$this._items = items;

  String? _nextCursor;
  String? get nextCursor => _$this._nextCursor;
  set nextCursor(String? nextCursor) => _$this._nextCursor = nextCursor;

  ApplicationGrantListResponseBuilder() {
    ApplicationGrantListResponse._defaults(this);
  }

  ApplicationGrantListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _nextCursor = $v.nextCursor;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationGrantListResponse other) {
    _$v = other as _$ApplicationGrantListResponse;
  }

  @override
  void update(void Function(ApplicationGrantListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationGrantListResponse build() => _build();

  _$ApplicationGrantListResponse _build() {
    _$ApplicationGrantListResponse _$result;
    try {
      _$result =
          _$v ??
          _$ApplicationGrantListResponse._(
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
          r'ApplicationGrantListResponse',
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
