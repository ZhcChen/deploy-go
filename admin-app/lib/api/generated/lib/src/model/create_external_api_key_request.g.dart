// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_external_api_key_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateExternalApiKeyRequest extends CreateExternalApiKeyRequest {
  @override
  final BuiltList<String> applicationIds;
  @override
  final String? expiresAt;
  @override
  final String name;

  factory _$CreateExternalApiKeyRequest([
    void Function(CreateExternalApiKeyRequestBuilder)? updates,
  ]) => (CreateExternalApiKeyRequestBuilder()..update(updates))._build();

  _$CreateExternalApiKeyRequest._({
    required this.applicationIds,
    this.expiresAt,
    required this.name,
  }) : super._();
  @override
  CreateExternalApiKeyRequest rebuild(
    void Function(CreateExternalApiKeyRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  CreateExternalApiKeyRequestBuilder toBuilder() =>
      CreateExternalApiKeyRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateExternalApiKeyRequest &&
        applicationIds == other.applicationIds &&
        expiresAt == other.expiresAt &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationIds.hashCode);
    _$hash = $jc(_$hash, expiresAt.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreateExternalApiKeyRequest')
          ..add('applicationIds', applicationIds)
          ..add('expiresAt', expiresAt)
          ..add('name', name))
        .toString();
  }
}

class CreateExternalApiKeyRequestBuilder
    implements
        Builder<
          CreateExternalApiKeyRequest,
          CreateExternalApiKeyRequestBuilder
        > {
  _$CreateExternalApiKeyRequest? _$v;

  ListBuilder<String>? _applicationIds;
  ListBuilder<String> get applicationIds =>
      _$this._applicationIds ??= ListBuilder<String>();
  set applicationIds(ListBuilder<String>? applicationIds) =>
      _$this._applicationIds = applicationIds;

  String? _expiresAt;
  String? get expiresAt => _$this._expiresAt;
  set expiresAt(String? expiresAt) => _$this._expiresAt = expiresAt;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  CreateExternalApiKeyRequestBuilder() {
    CreateExternalApiKeyRequest._defaults(this);
  }

  CreateExternalApiKeyRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationIds = $v.applicationIds.toBuilder();
      _expiresAt = $v.expiresAt;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateExternalApiKeyRequest other) {
    _$v = other as _$CreateExternalApiKeyRequest;
  }

  @override
  void update(void Function(CreateExternalApiKeyRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateExternalApiKeyRequest build() => _build();

  _$CreateExternalApiKeyRequest _build() {
    _$CreateExternalApiKeyRequest _$result;
    try {
      _$result =
          _$v ??
          _$CreateExternalApiKeyRequest._(
            applicationIds: applicationIds.build(),
            expiresAt: expiresAt,
            name: BuiltValueNullFieldError.checkNotNull(
              name,
              r'CreateExternalApiKeyRequest',
              'name',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'applicationIds';
        applicationIds.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'CreateExternalApiKeyRequest',
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
