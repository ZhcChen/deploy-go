// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_tag_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationTagListResponse extends ApplicationTagListResponse {
  @override
  final BuiltList<String> tags;

  factory _$ApplicationTagListResponse([
    void Function(ApplicationTagListResponseBuilder)? updates,
  ]) => (ApplicationTagListResponseBuilder()..update(updates))._build();

  _$ApplicationTagListResponse._({required this.tags}) : super._();
  @override
  ApplicationTagListResponse rebuild(
    void Function(ApplicationTagListResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationTagListResponseBuilder toBuilder() =>
      ApplicationTagListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationTagListResponse && tags == other.tags;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, tags.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'ApplicationTagListResponse',
    )..add('tags', tags)).toString();
  }
}

class ApplicationTagListResponseBuilder
    implements
        Builder<ApplicationTagListResponse, ApplicationTagListResponseBuilder> {
  _$ApplicationTagListResponse? _$v;

  ListBuilder<String>? _tags;
  ListBuilder<String> get tags => _$this._tags ??= ListBuilder<String>();
  set tags(ListBuilder<String>? tags) => _$this._tags = tags;

  ApplicationTagListResponseBuilder() {
    ApplicationTagListResponse._defaults(this);
  }

  ApplicationTagListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _tags = $v.tags.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationTagListResponse other) {
    _$v = other as _$ApplicationTagListResponse;
  }

  @override
  void update(void Function(ApplicationTagListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationTagListResponse build() => _build();

  _$ApplicationTagListResponse _build() {
    _$ApplicationTagListResponse _$result;
    try {
      _$result = _$v ?? _$ApplicationTagListResponse._(tags: tags.build());
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'tags';
        tags.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ApplicationTagListResponse',
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
