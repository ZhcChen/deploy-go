// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'save_application_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SaveApplicationRequest extends SaveApplicationRequest {
  @override
  final String? appType;
  @override
  final String? description;
  @override
  final String environment;
  @override
  final String name;
  @override
  final JsonObject? parameterSchema;
  @override
  final String slug;
  @override
  final BuiltList<String>? tags;
  @override
  final String? templateId;
  @override
  final String? typeVersion;
  @override
  final JsonObject? verificationConfig;
  @override
  final int? version;

  factory _$SaveApplicationRequest([
    void Function(SaveApplicationRequestBuilder)? updates,
  ]) => (SaveApplicationRequestBuilder()..update(updates))._build();

  _$SaveApplicationRequest._({
    this.appType,
    this.description,
    required this.environment,
    required this.name,
    this.parameterSchema,
    required this.slug,
    this.tags,
    this.templateId,
    this.typeVersion,
    this.verificationConfig,
    this.version,
  }) : super._();
  @override
  SaveApplicationRequest rebuild(
    void Function(SaveApplicationRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  SaveApplicationRequestBuilder toBuilder() =>
      SaveApplicationRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SaveApplicationRequest &&
        appType == other.appType &&
        description == other.description &&
        environment == other.environment &&
        name == other.name &&
        parameterSchema == other.parameterSchema &&
        slug == other.slug &&
        tags == other.tags &&
        templateId == other.templateId &&
        typeVersion == other.typeVersion &&
        verificationConfig == other.verificationConfig &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, appType.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, environment.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, parameterSchema.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jc(_$hash, tags.hashCode);
    _$hash = $jc(_$hash, templateId.hashCode);
    _$hash = $jc(_$hash, typeVersion.hashCode);
    _$hash = $jc(_$hash, verificationConfig.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SaveApplicationRequest')
          ..add('appType', appType)
          ..add('description', description)
          ..add('environment', environment)
          ..add('name', name)
          ..add('parameterSchema', parameterSchema)
          ..add('slug', slug)
          ..add('tags', tags)
          ..add('templateId', templateId)
          ..add('typeVersion', typeVersion)
          ..add('verificationConfig', verificationConfig)
          ..add('version', version))
        .toString();
  }
}

class SaveApplicationRequestBuilder
    implements Builder<SaveApplicationRequest, SaveApplicationRequestBuilder> {
  _$SaveApplicationRequest? _$v;

  String? _appType;
  String? get appType => _$this._appType;
  set appType(String? appType) => _$this._appType = appType;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _environment;
  String? get environment => _$this._environment;
  set environment(String? environment) => _$this._environment = environment;

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

  ListBuilder<String>? _tags;
  ListBuilder<String> get tags => _$this._tags ??= ListBuilder<String>();
  set tags(ListBuilder<String>? tags) => _$this._tags = tags;

  String? _templateId;
  String? get templateId => _$this._templateId;
  set templateId(String? templateId) => _$this._templateId = templateId;

  String? _typeVersion;
  String? get typeVersion => _$this._typeVersion;
  set typeVersion(String? typeVersion) => _$this._typeVersion = typeVersion;

  JsonObject? _verificationConfig;
  JsonObject? get verificationConfig => _$this._verificationConfig;
  set verificationConfig(JsonObject? verificationConfig) =>
      _$this._verificationConfig = verificationConfig;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  SaveApplicationRequestBuilder() {
    SaveApplicationRequest._defaults(this);
  }

  SaveApplicationRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _appType = $v.appType;
      _description = $v.description;
      _environment = $v.environment;
      _name = $v.name;
      _parameterSchema = $v.parameterSchema;
      _slug = $v.slug;
      _tags = $v.tags?.toBuilder();
      _templateId = $v.templateId;
      _typeVersion = $v.typeVersion;
      _verificationConfig = $v.verificationConfig;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SaveApplicationRequest other) {
    _$v = other as _$SaveApplicationRequest;
  }

  @override
  void update(void Function(SaveApplicationRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SaveApplicationRequest build() => _build();

  _$SaveApplicationRequest _build() {
    _$SaveApplicationRequest _$result;
    try {
      _$result =
          _$v ??
          _$SaveApplicationRequest._(
            appType: appType,
            description: description,
            environment: BuiltValueNullFieldError.checkNotNull(
              environment,
              r'SaveApplicationRequest',
              'environment',
            ),
            name: BuiltValueNullFieldError.checkNotNull(
              name,
              r'SaveApplicationRequest',
              'name',
            ),
            parameterSchema: parameterSchema,
            slug: BuiltValueNullFieldError.checkNotNull(
              slug,
              r'SaveApplicationRequest',
              'slug',
            ),
            tags: _tags?.build(),
            templateId: templateId,
            typeVersion: typeVersion,
            verificationConfig: verificationConfig,
            version: version,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'tags';
        _tags?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'SaveApplicationRequest',
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
