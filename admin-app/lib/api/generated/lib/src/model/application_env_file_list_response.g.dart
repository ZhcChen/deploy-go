// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_env_file_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationEnvFileListResponse extends ApplicationEnvFileListResponse {
  @override
  final BuiltList<ApplicationEnvFileResponse> items;

  factory _$ApplicationEnvFileListResponse([
    void Function(ApplicationEnvFileListResponseBuilder)? updates,
  ]) => (ApplicationEnvFileListResponseBuilder()..update(updates))._build();

  _$ApplicationEnvFileListResponse._({required this.items}) : super._();
  @override
  ApplicationEnvFileListResponse rebuild(
    void Function(ApplicationEnvFileListResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationEnvFileListResponseBuilder toBuilder() =>
      ApplicationEnvFileListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationEnvFileListResponse && items == other.items;
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
      r'ApplicationEnvFileListResponse',
    )..add('items', items)).toString();
  }
}

class ApplicationEnvFileListResponseBuilder
    implements
        Builder<
          ApplicationEnvFileListResponse,
          ApplicationEnvFileListResponseBuilder
        > {
  _$ApplicationEnvFileListResponse? _$v;

  ListBuilder<ApplicationEnvFileResponse>? _items;
  ListBuilder<ApplicationEnvFileResponse> get items =>
      _$this._items ??= ListBuilder<ApplicationEnvFileResponse>();
  set items(ListBuilder<ApplicationEnvFileResponse>? items) =>
      _$this._items = items;

  ApplicationEnvFileListResponseBuilder() {
    ApplicationEnvFileListResponse._defaults(this);
  }

  ApplicationEnvFileListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationEnvFileListResponse other) {
    _$v = other as _$ApplicationEnvFileListResponse;
  }

  @override
  void update(void Function(ApplicationEnvFileListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationEnvFileListResponse build() => _build();

  _$ApplicationEnvFileListResponse _build() {
    _$ApplicationEnvFileListResponse _$result;
    try {
      _$result =
          _$v ?? _$ApplicationEnvFileListResponse._(items: items.build());
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'items';
        items.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ApplicationEnvFileListResponse',
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
