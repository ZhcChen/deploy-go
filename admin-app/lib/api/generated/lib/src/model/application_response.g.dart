// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationResponse extends ApplicationResponse {
  @override
  final String appType;
  @override
  final String createdAt;
  @override
  final String description;
  @override
  final String environment;
  @override
  final String id;
  @override
  final String name;
  @override
  final JsonObject? parameterSchema;
  @override
  final String slug;
  @override
  final String status;
  @override
  final String typeVersion;
  @override
  final String updatedAt;
  @override
  final JsonObject? verificationConfig;
  @override
  final int version;

  factory _$ApplicationResponse([
    void Function(ApplicationResponseBuilder)? updates,
  ]) => (ApplicationResponseBuilder()..update(updates))._build();

  _$ApplicationResponse._({
    required this.appType,
    required this.createdAt,
    required this.description,
    required this.environment,
    required this.id,
    required this.name,
    this.parameterSchema,
    required this.slug,
    required this.status,
    required this.typeVersion,
    required this.updatedAt,
    this.verificationConfig,
    required this.version,
  }) : super._();
  @override
  ApplicationResponse rebuild(
    void Function(ApplicationResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationResponseBuilder toBuilder() =>
      ApplicationResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationResponse &&
        appType == other.appType &&
        createdAt == other.createdAt &&
        description == other.description &&
        environment == other.environment &&
        id == other.id &&
        name == other.name &&
        parameterSchema == other.parameterSchema &&
        slug == other.slug &&
        status == other.status &&
        typeVersion == other.typeVersion &&
        updatedAt == other.updatedAt &&
        verificationConfig == other.verificationConfig &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, appType.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, environment.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, parameterSchema.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, typeVersion.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, verificationConfig.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApplicationResponse')
          ..add('appType', appType)
          ..add('createdAt', createdAt)
          ..add('description', description)
          ..add('environment', environment)
          ..add('id', id)
          ..add('name', name)
          ..add('parameterSchema', parameterSchema)
          ..add('slug', slug)
          ..add('status', status)
          ..add('typeVersion', typeVersion)
          ..add('updatedAt', updatedAt)
          ..add('verificationConfig', verificationConfig)
          ..add('version', version))
        .toString();
  }
}

class ApplicationResponseBuilder
    implements Builder<ApplicationResponse, ApplicationResponseBuilder> {
  _$ApplicationResponse? _$v;

  String? _appType;
  String? get appType => _$this._appType;
  set appType(String? appType) => _$this._appType = appType;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _environment;
  String? get environment => _$this._environment;
  set environment(String? environment) => _$this._environment = environment;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  JsonObject? _parameterSchema;
  JsonObject? get parameterSchema => _$this._parameterSchema;
  set parameterSchema(JsonObject? parameterSchema) =>
      _$this._parameterSchema = parameterSchema;

  String? _slug;
  String? get slug => _$this._slug;
  set slug(String? slug) => _$this._slug = slug;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _typeVersion;
  String? get typeVersion => _$this._typeVersion;
  set typeVersion(String? typeVersion) => _$this._typeVersion = typeVersion;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  JsonObject? _verificationConfig;
  JsonObject? get verificationConfig => _$this._verificationConfig;
  set verificationConfig(JsonObject? verificationConfig) =>
      _$this._verificationConfig = verificationConfig;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  ApplicationResponseBuilder() {
    ApplicationResponse._defaults(this);
  }

  ApplicationResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _appType = $v.appType;
      _createdAt = $v.createdAt;
      _description = $v.description;
      _environment = $v.environment;
      _id = $v.id;
      _name = $v.name;
      _parameterSchema = $v.parameterSchema;
      _slug = $v.slug;
      _status = $v.status;
      _typeVersion = $v.typeVersion;
      _updatedAt = $v.updatedAt;
      _verificationConfig = $v.verificationConfig;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationResponse other) {
    _$v = other as _$ApplicationResponse;
  }

  @override
  void update(void Function(ApplicationResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationResponse build() => _build();

  _$ApplicationResponse _build() {
    final _$result =
        _$v ??
        _$ApplicationResponse._(
          appType: BuiltValueNullFieldError.checkNotNull(
            appType,
            r'ApplicationResponse',
            'appType',
          ),
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'ApplicationResponse',
            'createdAt',
          ),
          description: BuiltValueNullFieldError.checkNotNull(
            description,
            r'ApplicationResponse',
            'description',
          ),
          environment: BuiltValueNullFieldError.checkNotNull(
            environment,
            r'ApplicationResponse',
            'environment',
          ),
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'ApplicationResponse',
            'id',
          ),
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'ApplicationResponse',
            'name',
          ),
          parameterSchema: parameterSchema,
          slug: BuiltValueNullFieldError.checkNotNull(
            slug,
            r'ApplicationResponse',
            'slug',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'ApplicationResponse',
            'status',
          ),
          typeVersion: BuiltValueNullFieldError.checkNotNull(
            typeVersion,
            r'ApplicationResponse',
            'typeVersion',
          ),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
            updatedAt,
            r'ApplicationResponse',
            'updatedAt',
          ),
          verificationConfig: verificationConfig,
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'ApplicationResponse',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
