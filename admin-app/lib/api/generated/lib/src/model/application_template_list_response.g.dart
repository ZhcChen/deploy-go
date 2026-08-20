// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_template_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationTemplateListResponse
    extends ApplicationTemplateListResponse {
  @override
  final BuiltList<ApplicationTemplateResponse> items;

  factory _$ApplicationTemplateListResponse([
    void Function(ApplicationTemplateListResponseBuilder)? updates,
  ]) => (ApplicationTemplateListResponseBuilder()..update(updates))._build();

  _$ApplicationTemplateListResponse._({required this.items}) : super._();
  @override
  ApplicationTemplateListResponse rebuild(
    void Function(ApplicationTemplateListResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationTemplateListResponseBuilder toBuilder() =>
      ApplicationTemplateListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationTemplateListResponse && items == other.items;
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
      r'ApplicationTemplateListResponse',
    )..add('items', items)).toString();
  }
}

class ApplicationTemplateListResponseBuilder
    implements
        Builder<
          ApplicationTemplateListResponse,
          ApplicationTemplateListResponseBuilder
        > {
  _$ApplicationTemplateListResponse? _$v;

  ListBuilder<ApplicationTemplateResponse>? _items;
  ListBuilder<ApplicationTemplateResponse> get items =>
      _$this._items ??= ListBuilder<ApplicationTemplateResponse>();
  set items(ListBuilder<ApplicationTemplateResponse>? items) =>
      _$this._items = items;

  ApplicationTemplateListResponseBuilder() {
    ApplicationTemplateListResponse._defaults(this);
  }

  ApplicationTemplateListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationTemplateListResponse other) {
    _$v = other as _$ApplicationTemplateListResponse;
  }

  @override
  void update(void Function(ApplicationTemplateListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationTemplateListResponse build() => _build();

  _$ApplicationTemplateListResponse _build() {
    _$ApplicationTemplateListResponse _$result;
    try {
      _$result =
          _$v ?? _$ApplicationTemplateListResponse._(items: items.build());
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'items';
        items.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ApplicationTemplateListResponse',
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
