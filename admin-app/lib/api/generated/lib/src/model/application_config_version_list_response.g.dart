// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_config_version_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationConfigVersionListResponse
    extends ApplicationConfigVersionListResponse {
  @override
  final BuiltList<ApplicationConfigVersionResponse> items;

  factory _$ApplicationConfigVersionListResponse([
    void Function(ApplicationConfigVersionListResponseBuilder)? updates,
  ]) =>
      (ApplicationConfigVersionListResponseBuilder()..update(updates))._build();

  _$ApplicationConfigVersionListResponse._({required this.items}) : super._();
  @override
  ApplicationConfigVersionListResponse rebuild(
    void Function(ApplicationConfigVersionListResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationConfigVersionListResponseBuilder toBuilder() =>
      ApplicationConfigVersionListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationConfigVersionListResponse &&
        items == other.items;
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
      r'ApplicationConfigVersionListResponse',
    )..add('items', items)).toString();
  }
}

class ApplicationConfigVersionListResponseBuilder
    implements
        Builder<
          ApplicationConfigVersionListResponse,
          ApplicationConfigVersionListResponseBuilder
        > {
  _$ApplicationConfigVersionListResponse? _$v;

  ListBuilder<ApplicationConfigVersionResponse>? _items;
  ListBuilder<ApplicationConfigVersionResponse> get items =>
      _$this._items ??= ListBuilder<ApplicationConfigVersionResponse>();
  set items(ListBuilder<ApplicationConfigVersionResponse>? items) =>
      _$this._items = items;

  ApplicationConfigVersionListResponseBuilder() {
    ApplicationConfigVersionListResponse._defaults(this);
  }

  ApplicationConfigVersionListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationConfigVersionListResponse other) {
    _$v = other as _$ApplicationConfigVersionListResponse;
  }

  @override
  void update(
    void Function(ApplicationConfigVersionListResponseBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationConfigVersionListResponse build() => _build();

  _$ApplicationConfigVersionListResponse _build() {
    _$ApplicationConfigVersionListResponse _$result;
    try {
      _$result =
          _$v ?? _$ApplicationConfigVersionListResponse._(items: items.build());
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'items';
        items.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ApplicationConfigVersionListResponse',
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
