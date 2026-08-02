// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'save_target_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SaveTargetRequest extends SaveTargetRequest {
  @override
  final String environment;
  @override
  final String nodeId;
  @override
  final JsonObject? parameterSchema;
  @override
  final String scriptPath;
  @override
  final BuiltList<SecretFileReference>? secretFileReferences;
  @override
  final int timeoutSeconds;
  @override
  final JsonObject? verificationConfig;
  @override
  final int? version;

  factory _$SaveTargetRequest(
          [void Function(SaveTargetRequestBuilder)? updates]) =>
      (SaveTargetRequestBuilder()..update(updates))._build();

  _$SaveTargetRequest._(
      {required this.environment,
      required this.nodeId,
      this.parameterSchema,
      required this.scriptPath,
      this.secretFileReferences,
      required this.timeoutSeconds,
      this.verificationConfig,
      this.version})
      : super._();
  @override
  SaveTargetRequest rebuild(void Function(SaveTargetRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SaveTargetRequestBuilder toBuilder() =>
      SaveTargetRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SaveTargetRequest &&
        environment == other.environment &&
        nodeId == other.nodeId &&
        parameterSchema == other.parameterSchema &&
        scriptPath == other.scriptPath &&
        secretFileReferences == other.secretFileReferences &&
        timeoutSeconds == other.timeoutSeconds &&
        verificationConfig == other.verificationConfig &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, environment.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, parameterSchema.hashCode);
    _$hash = $jc(_$hash, scriptPath.hashCode);
    _$hash = $jc(_$hash, secretFileReferences.hashCode);
    _$hash = $jc(_$hash, timeoutSeconds.hashCode);
    _$hash = $jc(_$hash, verificationConfig.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SaveTargetRequest')
          ..add('environment', environment)
          ..add('nodeId', nodeId)
          ..add('parameterSchema', parameterSchema)
          ..add('scriptPath', scriptPath)
          ..add('secretFileReferences', secretFileReferences)
          ..add('timeoutSeconds', timeoutSeconds)
          ..add('verificationConfig', verificationConfig)
          ..add('version', version))
        .toString();
  }
}

class SaveTargetRequestBuilder
    implements Builder<SaveTargetRequest, SaveTargetRequestBuilder> {
  _$SaveTargetRequest? _$v;

  String? _environment;
  String? get environment => _$this._environment;
  set environment(String? environment) => _$this._environment = environment;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  JsonObject? _parameterSchema;
  JsonObject? get parameterSchema => _$this._parameterSchema;
  set parameterSchema(JsonObject? parameterSchema) =>
      _$this._parameterSchema = parameterSchema;

  String? _scriptPath;
  String? get scriptPath => _$this._scriptPath;
  set scriptPath(String? scriptPath) => _$this._scriptPath = scriptPath;

  ListBuilder<SecretFileReference>? _secretFileReferences;
  ListBuilder<SecretFileReference> get secretFileReferences =>
      _$this._secretFileReferences ??= ListBuilder<SecretFileReference>();
  set secretFileReferences(
          ListBuilder<SecretFileReference>? secretFileReferences) =>
      _$this._secretFileReferences = secretFileReferences;

  int? _timeoutSeconds;
  int? get timeoutSeconds => _$this._timeoutSeconds;
  set timeoutSeconds(int? timeoutSeconds) =>
      _$this._timeoutSeconds = timeoutSeconds;

  JsonObject? _verificationConfig;
  JsonObject? get verificationConfig => _$this._verificationConfig;
  set verificationConfig(JsonObject? verificationConfig) =>
      _$this._verificationConfig = verificationConfig;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  SaveTargetRequestBuilder() {
    SaveTargetRequest._defaults(this);
  }

  SaveTargetRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _environment = $v.environment;
      _nodeId = $v.nodeId;
      _parameterSchema = $v.parameterSchema;
      _scriptPath = $v.scriptPath;
      _secretFileReferences = $v.secretFileReferences?.toBuilder();
      _timeoutSeconds = $v.timeoutSeconds;
      _verificationConfig = $v.verificationConfig;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SaveTargetRequest other) {
    _$v = other as _$SaveTargetRequest;
  }

  @override
  void update(void Function(SaveTargetRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SaveTargetRequest build() => _build();

  _$SaveTargetRequest _build() {
    _$SaveTargetRequest _$result;
    try {
      _$result = _$v ??
          _$SaveTargetRequest._(
            environment: BuiltValueNullFieldError.checkNotNull(
                environment, r'SaveTargetRequest', 'environment'),
            nodeId: BuiltValueNullFieldError.checkNotNull(
                nodeId, r'SaveTargetRequest', 'nodeId'),
            parameterSchema: parameterSchema,
            scriptPath: BuiltValueNullFieldError.checkNotNull(
                scriptPath, r'SaveTargetRequest', 'scriptPath'),
            secretFileReferences: _secretFileReferences?.build(),
            timeoutSeconds: BuiltValueNullFieldError.checkNotNull(
                timeoutSeconds, r'SaveTargetRequest', 'timeoutSeconds'),
            verificationConfig: verificationConfig,
            version: version,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'secretFileReferences';
        _secretFileReferences?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'SaveTargetRequest', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
