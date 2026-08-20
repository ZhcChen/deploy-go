// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_config_file_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationConfigFileListResponse
    extends ApplicationConfigFileListResponse {
  @override
  final BuiltList<ApplicationConfigFileResponse> items;

  factory _$ApplicationConfigFileListResponse([
    void Function(ApplicationConfigFileListResponseBuilder)? updates,
  ]) => (ApplicationConfigFileListResponseBuilder()..update(updates))._build();

  _$ApplicationConfigFileListResponse._({required this.items}) : super._();
  @override
  ApplicationConfigFileListResponse rebuild(
    void Function(ApplicationConfigFileListResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationConfigFileListResponseBuilder toBuilder() =>
      ApplicationConfigFileListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationConfigFileListResponse && items == other.items;
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
      r'ApplicationConfigFileListResponse',
    )..add('items', items)).toString();
  }
}

class ApplicationConfigFileListResponseBuilder
    implements
        Builder<
          ApplicationConfigFileListResponse,
          ApplicationConfigFileListResponseBuilder
        > {
  _$ApplicationConfigFileListResponse? _$v;

  ListBuilder<ApplicationConfigFileResponse>? _items;
  ListBuilder<ApplicationConfigFileResponse> get items =>
      _$this._items ??= ListBuilder<ApplicationConfigFileResponse>();
  set items(ListBuilder<ApplicationConfigFileResponse>? items) =>
      _$this._items = items;

  ApplicationConfigFileListResponseBuilder() {
    ApplicationConfigFileListResponse._defaults(this);
  }

  ApplicationConfigFileListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationConfigFileListResponse other) {
    _$v = other as _$ApplicationConfigFileListResponse;
  }

  @override
  void update(
    void Function(ApplicationConfigFileListResponseBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationConfigFileListResponse build() => _build();

  _$ApplicationConfigFileListResponse _build() {
    _$ApplicationConfigFileListResponse _$result;
    try {
      _$result =
          _$v ?? _$ApplicationConfigFileListResponse._(items: items.build());
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'items';
        items.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ApplicationConfigFileListResponse',
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
