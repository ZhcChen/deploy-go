// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'deployment_target_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeploymentTargetResponse extends DeploymentTargetResponse {
  @override
  final String applicationId;
  @override
  final String createdAt;
  @override
  final String environment;
  @override
  final String executionMode;
  @override
  final String id;
  @override
  final ImageDeploySpec? imageSpec;
  @override
  final String nodeId;
  @override
  final JsonObject? parameterSchema;
  @override
  final bool privilegedRelease;
  @override
  final String scriptPath;
  @override
  final BuiltList<SecretFileReference> secretFileReferences;
  @override
  final String snapshotHash;
  @override
  final String status;
  @override
  final int timeoutSeconds;
  @override
  final String updatedAt;
  @override
  final JsonObject? verificationConfig;
  @override
  final int version;

  factory _$DeploymentTargetResponse([
    void Function(DeploymentTargetResponseBuilder)? updates,
  ]) => (DeploymentTargetResponseBuilder()..update(updates))._build();

  _$DeploymentTargetResponse._({
    required this.applicationId,
    required this.createdAt,
    required this.environment,
    required this.executionMode,
    required this.id,
    this.imageSpec,
    required this.nodeId,
    this.parameterSchema,
    required this.privilegedRelease,
    required this.scriptPath,
    required this.secretFileReferences,
    required this.snapshotHash,
    required this.status,
    required this.timeoutSeconds,
    required this.updatedAt,
    this.verificationConfig,
    required this.version,
  }) : super._();
  @override
  DeploymentTargetResponse rebuild(
    void Function(DeploymentTargetResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  DeploymentTargetResponseBuilder toBuilder() =>
      DeploymentTargetResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeploymentTargetResponse &&
        applicationId == other.applicationId &&
        createdAt == other.createdAt &&
        environment == other.environment &&
        executionMode == other.executionMode &&
        id == other.id &&
        imageSpec == other.imageSpec &&
        nodeId == other.nodeId &&
        parameterSchema == other.parameterSchema &&
        privilegedRelease == other.privilegedRelease &&
        scriptPath == other.scriptPath &&
        secretFileReferences == other.secretFileReferences &&
        snapshotHash == other.snapshotHash &&
        status == other.status &&
        timeoutSeconds == other.timeoutSeconds &&
        updatedAt == other.updatedAt &&
        verificationConfig == other.verificationConfig &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationId.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, environment.hashCode);
    _$hash = $jc(_$hash, executionMode.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, imageSpec.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, parameterSchema.hashCode);
    _$hash = $jc(_$hash, privilegedRelease.hashCode);
    _$hash = $jc(_$hash, scriptPath.hashCode);
    _$hash = $jc(_$hash, secretFileReferences.hashCode);
    _$hash = $jc(_$hash, snapshotHash.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, timeoutSeconds.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, verificationConfig.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'DeploymentTargetResponse')
          ..add('applicationId', applicationId)
          ..add('createdAt', createdAt)
          ..add('environment', environment)
          ..add('executionMode', executionMode)
          ..add('id', id)
          ..add('imageSpec', imageSpec)
          ..add('nodeId', nodeId)
          ..add('parameterSchema', parameterSchema)
          ..add('privilegedRelease', privilegedRelease)
          ..add('scriptPath', scriptPath)
          ..add('secretFileReferences', secretFileReferences)
          ..add('snapshotHash', snapshotHash)
          ..add('status', status)
          ..add('timeoutSeconds', timeoutSeconds)
          ..add('updatedAt', updatedAt)
          ..add('verificationConfig', verificationConfig)
          ..add('version', version))
        .toString();
  }
}

class DeploymentTargetResponseBuilder
    implements
        Builder<DeploymentTargetResponse, DeploymentTargetResponseBuilder> {
  _$DeploymentTargetResponse? _$v;

  String? _applicationId;
  String? get applicationId => _$this._applicationId;
  set applicationId(String? applicationId) =>
      _$this._applicationId = applicationId;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _environment;
  String? get environment => _$this._environment;
  set environment(String? environment) => _$this._environment = environment;

  String? _executionMode;
  String? get executionMode => _$this._executionMode;
  set executionMode(String? executionMode) =>
      _$this._executionMode = executionMode;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  ImageDeploySpecBuilder? _imageSpec;
  ImageDeploySpecBuilder get imageSpec =>
      _$this._imageSpec ??= ImageDeploySpecBuilder();
  set imageSpec(ImageDeploySpecBuilder? imageSpec) =>
      _$this._imageSpec = imageSpec;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  JsonObject? _parameterSchema;
  JsonObject? get parameterSchema => _$this._parameterSchema;
  set parameterSchema(JsonObject? parameterSchema) =>
      _$this._parameterSchema = parameterSchema;

  bool? _privilegedRelease;
  bool? get privilegedRelease => _$this._privilegedRelease;
  set privilegedRelease(bool? privilegedRelease) =>
      _$this._privilegedRelease = privilegedRelease;

  String? _scriptPath;
  String? get scriptPath => _$this._scriptPath;
  set scriptPath(String? scriptPath) => _$this._scriptPath = scriptPath;

  ListBuilder<SecretFileReference>? _secretFileReferences;
  ListBuilder<SecretFileReference> get secretFileReferences =>
      _$this._secretFileReferences ??= ListBuilder<SecretFileReference>();
  set secretFileReferences(
    ListBuilder<SecretFileReference>? secretFileReferences,
  ) => _$this._secretFileReferences = secretFileReferences;

  String? _snapshotHash;
  String? get snapshotHash => _$this._snapshotHash;
  set snapshotHash(String? snapshotHash) => _$this._snapshotHash = snapshotHash;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  int? _timeoutSeconds;
  int? get timeoutSeconds => _$this._timeoutSeconds;
  set timeoutSeconds(int? timeoutSeconds) =>
      _$this._timeoutSeconds = timeoutSeconds;

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

  DeploymentTargetResponseBuilder() {
    DeploymentTargetResponse._defaults(this);
  }

  DeploymentTargetResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationId = $v.applicationId;
      _createdAt = $v.createdAt;
      _environment = $v.environment;
      _executionMode = $v.executionMode;
      _id = $v.id;
      _imageSpec = $v.imageSpec?.toBuilder();
      _nodeId = $v.nodeId;
      _parameterSchema = $v.parameterSchema;
      _privilegedRelease = $v.privilegedRelease;
      _scriptPath = $v.scriptPath;
      _secretFileReferences = $v.secretFileReferences.toBuilder();
      _snapshotHash = $v.snapshotHash;
      _status = $v.status;
      _timeoutSeconds = $v.timeoutSeconds;
      _updatedAt = $v.updatedAt;
      _verificationConfig = $v.verificationConfig;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeploymentTargetResponse other) {
    _$v = other as _$DeploymentTargetResponse;
  }

  @override
  void update(void Function(DeploymentTargetResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeploymentTargetResponse build() => _build();

  _$DeploymentTargetResponse _build() {
    _$DeploymentTargetResponse _$result;
    try {
      _$result =
          _$v ??
          _$DeploymentTargetResponse._(
            applicationId: BuiltValueNullFieldError.checkNotNull(
              applicationId,
              r'DeploymentTargetResponse',
              'applicationId',
            ),
            createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt,
              r'DeploymentTargetResponse',
              'createdAt',
            ),
            environment: BuiltValueNullFieldError.checkNotNull(
              environment,
              r'DeploymentTargetResponse',
              'environment',
            ),
            executionMode: BuiltValueNullFieldError.checkNotNull(
              executionMode,
              r'DeploymentTargetResponse',
              'executionMode',
            ),
            id: BuiltValueNullFieldError.checkNotNull(
              id,
              r'DeploymentTargetResponse',
              'id',
            ),
            imageSpec: _imageSpec?.build(),
            nodeId: BuiltValueNullFieldError.checkNotNull(
              nodeId,
              r'DeploymentTargetResponse',
              'nodeId',
            ),
            parameterSchema: parameterSchema,
            privilegedRelease: BuiltValueNullFieldError.checkNotNull(
              privilegedRelease,
              r'DeploymentTargetResponse',
              'privilegedRelease',
            ),
            scriptPath: BuiltValueNullFieldError.checkNotNull(
              scriptPath,
              r'DeploymentTargetResponse',
              'scriptPath',
            ),
            secretFileReferences: secretFileReferences.build(),
            snapshotHash: BuiltValueNullFieldError.checkNotNull(
              snapshotHash,
              r'DeploymentTargetResponse',
              'snapshotHash',
            ),
            status: BuiltValueNullFieldError.checkNotNull(
              status,
              r'DeploymentTargetResponse',
              'status',
            ),
            timeoutSeconds: BuiltValueNullFieldError.checkNotNull(
              timeoutSeconds,
              r'DeploymentTargetResponse',
              'timeoutSeconds',
            ),
            updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt,
              r'DeploymentTargetResponse',
              'updatedAt',
            ),
            verificationConfig: verificationConfig,
            version: BuiltValueNullFieldError.checkNotNull(
              version,
              r'DeploymentTargetResponse',
              'version',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'imageSpec';
        _imageSpec?.build();

        _$failedField = 'secretFileReferences';
        secretFileReferences.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'DeploymentTargetResponse',
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
