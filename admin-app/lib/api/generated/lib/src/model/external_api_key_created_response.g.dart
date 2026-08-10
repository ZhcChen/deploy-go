// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'external_api_key_created_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ExternalApiKeyCreatedResponse extends ExternalApiKeyCreatedResponse {
  @override
  final BuiltList<String> applicationIds;
  @override
  final String createdAt;
  @override
  final String? expiresAt;
  @override
  final String id;
  @override
  final String name;
  @override
  final String status;
  @override
  final String token;

  factory _$ExternalApiKeyCreatedResponse([
    void Function(ExternalApiKeyCreatedResponseBuilder)? updates,
  ]) => (ExternalApiKeyCreatedResponseBuilder()..update(updates))._build();

  _$ExternalApiKeyCreatedResponse._({
    required this.applicationIds,
    required this.createdAt,
    this.expiresAt,
    required this.id,
    required this.name,
    required this.status,
    required this.token,
  }) : super._();
  @override
  ExternalApiKeyCreatedResponse rebuild(
    void Function(ExternalApiKeyCreatedResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ExternalApiKeyCreatedResponseBuilder toBuilder() =>
      ExternalApiKeyCreatedResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ExternalApiKeyCreatedResponse &&
        applicationIds == other.applicationIds &&
        createdAt == other.createdAt &&
        expiresAt == other.expiresAt &&
        id == other.id &&
        name == other.name &&
        status == other.status &&
        token == other.token;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationIds.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, expiresAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, token.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ExternalApiKeyCreatedResponse')
          ..add('applicationIds', applicationIds)
          ..add('createdAt', createdAt)
          ..add('expiresAt', expiresAt)
          ..add('id', id)
          ..add('name', name)
          ..add('status', status)
          ..add('token', token))
        .toString();
  }
}

class ExternalApiKeyCreatedResponseBuilder
    implements
        Builder<
          ExternalApiKeyCreatedResponse,
          ExternalApiKeyCreatedResponseBuilder
        > {
  _$ExternalApiKeyCreatedResponse? _$v;

  ListBuilder<String>? _applicationIds;
  ListBuilder<String> get applicationIds =>
      _$this._applicationIds ??= ListBuilder<String>();
  set applicationIds(ListBuilder<String>? applicationIds) =>
      _$this._applicationIds = applicationIds;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _expiresAt;
  String? get expiresAt => _$this._expiresAt;
  set expiresAt(String? expiresAt) => _$this._expiresAt = expiresAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _token;
  String? get token => _$this._token;
  set token(String? token) => _$this._token = token;

  ExternalApiKeyCreatedResponseBuilder() {
    ExternalApiKeyCreatedResponse._defaults(this);
  }

  ExternalApiKeyCreatedResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationIds = $v.applicationIds.toBuilder();
      _createdAt = $v.createdAt;
      _expiresAt = $v.expiresAt;
      _id = $v.id;
      _name = $v.name;
      _status = $v.status;
      _token = $v.token;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ExternalApiKeyCreatedResponse other) {
    _$v = other as _$ExternalApiKeyCreatedResponse;
  }

  @override
  void update(void Function(ExternalApiKeyCreatedResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ExternalApiKeyCreatedResponse build() => _build();

  _$ExternalApiKeyCreatedResponse _build() {
    _$ExternalApiKeyCreatedResponse _$result;
    try {
      _$result =
          _$v ??
          _$ExternalApiKeyCreatedResponse._(
            applicationIds: applicationIds.build(),
            createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt,
              r'ExternalApiKeyCreatedResponse',
              'createdAt',
            ),
            expiresAt: expiresAt,
            id: BuiltValueNullFieldError.checkNotNull(
              id,
              r'ExternalApiKeyCreatedResponse',
              'id',
            ),
            name: BuiltValueNullFieldError.checkNotNull(
              name,
              r'ExternalApiKeyCreatedResponse',
              'name',
            ),
            status: BuiltValueNullFieldError.checkNotNull(
              status,
              r'ExternalApiKeyCreatedResponse',
              'status',
            ),
            token: BuiltValueNullFieldError.checkNotNull(
              token,
              r'ExternalApiKeyCreatedResponse',
              'token',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'applicationIds';
        applicationIds.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ExternalApiKeyCreatedResponse',
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
