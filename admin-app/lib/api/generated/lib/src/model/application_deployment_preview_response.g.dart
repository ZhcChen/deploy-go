// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_deployment_preview_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationDeploymentPreviewResponse
    extends ApplicationDeploymentPreviewResponse {
  @override
  final String applicationId;
  @override
  final String applicationName;
  @override
  final String? deploymentBranch;
  @override
  final String executionMode;
  @override
  final JsonObject? imageSpec;
  @override
  final BuiltList<String>? modules;
  @override
  final JsonObject? parameters;
  @override
  final String? previewExpiresAt;
  @override
  final String releaseStrategy;
  @override
  final String? releaseVersion;
  @override
  final String? resolvedCommitSha;
  @override
  final String snapshotHash;
  @override
  final BuiltList<DeploymentTargetPreviewResponse> targets;
  @override
  final String? workspacePath;
  @override
  final int? workspaceVersion;

  factory _$ApplicationDeploymentPreviewResponse([
    void Function(ApplicationDeploymentPreviewResponseBuilder)? updates,
  ]) =>
      (ApplicationDeploymentPreviewResponseBuilder()..update(updates))._build();

  _$ApplicationDeploymentPreviewResponse._({
    required this.applicationId,
    required this.applicationName,
    this.deploymentBranch,
    required this.executionMode,
    this.imageSpec,
    this.modules,
    this.parameters,
    this.previewExpiresAt,
    required this.releaseStrategy,
    this.releaseVersion,
    this.resolvedCommitSha,
    required this.snapshotHash,
    required this.targets,
    this.workspacePath,
    this.workspaceVersion,
  }) : super._();
  @override
  ApplicationDeploymentPreviewResponse rebuild(
    void Function(ApplicationDeploymentPreviewResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationDeploymentPreviewResponseBuilder toBuilder() =>
      ApplicationDeploymentPreviewResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationDeploymentPreviewResponse &&
        applicationId == other.applicationId &&
        applicationName == other.applicationName &&
        deploymentBranch == other.deploymentBranch &&
        executionMode == other.executionMode &&
        imageSpec == other.imageSpec &&
        modules == other.modules &&
        parameters == other.parameters &&
        previewExpiresAt == other.previewExpiresAt &&
        releaseStrategy == other.releaseStrategy &&
        releaseVersion == other.releaseVersion &&
        resolvedCommitSha == other.resolvedCommitSha &&
        snapshotHash == other.snapshotHash &&
        targets == other.targets &&
        workspacePath == other.workspacePath &&
        workspaceVersion == other.workspaceVersion;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationId.hashCode);
    _$hash = $jc(_$hash, applicationName.hashCode);
    _$hash = $jc(_$hash, deploymentBranch.hashCode);
    _$hash = $jc(_$hash, executionMode.hashCode);
    _$hash = $jc(_$hash, imageSpec.hashCode);
    _$hash = $jc(_$hash, modules.hashCode);
    _$hash = $jc(_$hash, parameters.hashCode);
    _$hash = $jc(_$hash, previewExpiresAt.hashCode);
    _$hash = $jc(_$hash, releaseStrategy.hashCode);
    _$hash = $jc(_$hash, releaseVersion.hashCode);
    _$hash = $jc(_$hash, resolvedCommitSha.hashCode);
    _$hash = $jc(_$hash, snapshotHash.hashCode);
    _$hash = $jc(_$hash, targets.hashCode);
    _$hash = $jc(_$hash, workspacePath.hashCode);
    _$hash = $jc(_$hash, workspaceVersion.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApplicationDeploymentPreviewResponse')
          ..add('applicationId', applicationId)
          ..add('applicationName', applicationName)
          ..add('deploymentBranch', deploymentBranch)
          ..add('executionMode', executionMode)
          ..add('imageSpec', imageSpec)
          ..add('modules', modules)
          ..add('parameters', parameters)
          ..add('previewExpiresAt', previewExpiresAt)
          ..add('releaseStrategy', releaseStrategy)
          ..add('releaseVersion', releaseVersion)
          ..add('resolvedCommitSha', resolvedCommitSha)
          ..add('snapshotHash', snapshotHash)
          ..add('targets', targets)
          ..add('workspacePath', workspacePath)
          ..add('workspaceVersion', workspaceVersion))
        .toString();
  }
}

class ApplicationDeploymentPreviewResponseBuilder
    implements
        Builder<
          ApplicationDeploymentPreviewResponse,
          ApplicationDeploymentPreviewResponseBuilder
        > {
  _$ApplicationDeploymentPreviewResponse? _$v;

  String? _applicationId;
  String? get applicationId => _$this._applicationId;
  set applicationId(String? applicationId) =>
      _$this._applicationId = applicationId;

  String? _applicationName;
  String? get applicationName => _$this._applicationName;
  set applicationName(String? applicationName) =>
      _$this._applicationName = applicationName;

  String? _deploymentBranch;
  String? get deploymentBranch => _$this._deploymentBranch;
  set deploymentBranch(String? deploymentBranch) =>
      _$this._deploymentBranch = deploymentBranch;

  String? _executionMode;
  String? get executionMode => _$this._executionMode;
  set executionMode(String? executionMode) =>
      _$this._executionMode = executionMode;

  JsonObject? _imageSpec;
  JsonObject? get imageSpec => _$this._imageSpec;
  set imageSpec(JsonObject? imageSpec) => _$this._imageSpec = imageSpec;

  ListBuilder<String>? _modules;
  ListBuilder<String> get modules => _$this._modules ??= ListBuilder<String>();
  set modules(ListBuilder<String>? modules) => _$this._modules = modules;

  JsonObject? _parameters;
  JsonObject? get parameters => _$this._parameters;
  set parameters(JsonObject? parameters) => _$this._parameters = parameters;

  String? _previewExpiresAt;
  String? get previewExpiresAt => _$this._previewExpiresAt;
  set previewExpiresAt(String? previewExpiresAt) =>
      _$this._previewExpiresAt = previewExpiresAt;

  String? _releaseStrategy;
  String? get releaseStrategy => _$this._releaseStrategy;
  set releaseStrategy(String? releaseStrategy) =>
      _$this._releaseStrategy = releaseStrategy;

  String? _releaseVersion;
  String? get releaseVersion => _$this._releaseVersion;
  set releaseVersion(String? releaseVersion) =>
      _$this._releaseVersion = releaseVersion;

  String? _resolvedCommitSha;
  String? get resolvedCommitSha => _$this._resolvedCommitSha;
  set resolvedCommitSha(String? resolvedCommitSha) =>
      _$this._resolvedCommitSha = resolvedCommitSha;

  String? _snapshotHash;
  String? get snapshotHash => _$this._snapshotHash;
  set snapshotHash(String? snapshotHash) => _$this._snapshotHash = snapshotHash;

  ListBuilder<DeploymentTargetPreviewResponse>? _targets;
  ListBuilder<DeploymentTargetPreviewResponse> get targets =>
      _$this._targets ??= ListBuilder<DeploymentTargetPreviewResponse>();
  set targets(ListBuilder<DeploymentTargetPreviewResponse>? targets) =>
      _$this._targets = targets;

  String? _workspacePath;
  String? get workspacePath => _$this._workspacePath;
  set workspacePath(String? workspacePath) =>
      _$this._workspacePath = workspacePath;

  int? _workspaceVersion;
  int? get workspaceVersion => _$this._workspaceVersion;
  set workspaceVersion(int? workspaceVersion) =>
      _$this._workspaceVersion = workspaceVersion;

  ApplicationDeploymentPreviewResponseBuilder() {
    ApplicationDeploymentPreviewResponse._defaults(this);
  }

  ApplicationDeploymentPreviewResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationId = $v.applicationId;
      _applicationName = $v.applicationName;
      _deploymentBranch = $v.deploymentBranch;
      _executionMode = $v.executionMode;
      _imageSpec = $v.imageSpec;
      _modules = $v.modules?.toBuilder();
      _parameters = $v.parameters;
      _previewExpiresAt = $v.previewExpiresAt;
      _releaseStrategy = $v.releaseStrategy;
      _releaseVersion = $v.releaseVersion;
      _resolvedCommitSha = $v.resolvedCommitSha;
      _snapshotHash = $v.snapshotHash;
      _targets = $v.targets.toBuilder();
      _workspacePath = $v.workspacePath;
      _workspaceVersion = $v.workspaceVersion;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationDeploymentPreviewResponse other) {
    _$v = other as _$ApplicationDeploymentPreviewResponse;
  }

  @override
  void update(
    void Function(ApplicationDeploymentPreviewResponseBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationDeploymentPreviewResponse build() => _build();

  _$ApplicationDeploymentPreviewResponse _build() {
    _$ApplicationDeploymentPreviewResponse _$result;
    try {
      _$result =
          _$v ??
          _$ApplicationDeploymentPreviewResponse._(
            applicationId: BuiltValueNullFieldError.checkNotNull(
              applicationId,
              r'ApplicationDeploymentPreviewResponse',
              'applicationId',
            ),
            applicationName: BuiltValueNullFieldError.checkNotNull(
              applicationName,
              r'ApplicationDeploymentPreviewResponse',
              'applicationName',
            ),
            deploymentBranch: deploymentBranch,
            executionMode: BuiltValueNullFieldError.checkNotNull(
              executionMode,
              r'ApplicationDeploymentPreviewResponse',
              'executionMode',
            ),
            imageSpec: imageSpec,
            modules: _modules?.build(),
            parameters: parameters,
            previewExpiresAt: previewExpiresAt,
            releaseStrategy: BuiltValueNullFieldError.checkNotNull(
              releaseStrategy,
              r'ApplicationDeploymentPreviewResponse',
              'releaseStrategy',
            ),
            releaseVersion: releaseVersion,
            resolvedCommitSha: resolvedCommitSha,
            snapshotHash: BuiltValueNullFieldError.checkNotNull(
              snapshotHash,
              r'ApplicationDeploymentPreviewResponse',
              'snapshotHash',
            ),
            targets: targets.build(),
            workspacePath: workspacePath,
            workspaceVersion: workspaceVersion,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'modules';
        _modules?.build();

        _$failedField = 'targets';
        targets.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ApplicationDeploymentPreviewResponse',
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
