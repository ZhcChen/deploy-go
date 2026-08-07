// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'deployment_event_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeploymentEventResponse extends DeploymentEventResponse {
  @override
  final String createdAt;
  @override
  final String eventName;
  @override
  final String? failureStage;
  @override
  final String id;
  @override
  final String? message;
  @override
  final String? module;
  @override
  final String? moduleName;
  @override
  final String? stage;
  @override
  final String? status;
  @override
  final String? step;
  @override
  final String? stepId;

  factory _$DeploymentEventResponse([
    void Function(DeploymentEventResponseBuilder)? updates,
  ]) => (DeploymentEventResponseBuilder()..update(updates))._build();

  _$DeploymentEventResponse._({
    required this.createdAt,
    required this.eventName,
    this.failureStage,
    required this.id,
    this.message,
    this.module,
    this.moduleName,
    this.stage,
    this.status,
    this.step,
    this.stepId,
  }) : super._();
  @override
  DeploymentEventResponse rebuild(
    void Function(DeploymentEventResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  DeploymentEventResponseBuilder toBuilder() =>
      DeploymentEventResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeploymentEventResponse &&
        createdAt == other.createdAt &&
        eventName == other.eventName &&
        failureStage == other.failureStage &&
        id == other.id &&
        message == other.message &&
        module == other.module &&
        moduleName == other.moduleName &&
        stage == other.stage &&
        status == other.status &&
        step == other.step &&
        stepId == other.stepId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, eventName.hashCode);
    _$hash = $jc(_$hash, failureStage.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jc(_$hash, module.hashCode);
    _$hash = $jc(_$hash, moduleName.hashCode);
    _$hash = $jc(_$hash, stage.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, step.hashCode);
    _$hash = $jc(_$hash, stepId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'DeploymentEventResponse')
          ..add('createdAt', createdAt)
          ..add('eventName', eventName)
          ..add('failureStage', failureStage)
          ..add('id', id)
          ..add('message', message)
          ..add('module', module)
          ..add('moduleName', moduleName)
          ..add('stage', stage)
          ..add('status', status)
          ..add('step', step)
          ..add('stepId', stepId))
        .toString();
  }
}

class DeploymentEventResponseBuilder
    implements
        Builder<DeploymentEventResponse, DeploymentEventResponseBuilder> {
  _$DeploymentEventResponse? _$v;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _eventName;
  String? get eventName => _$this._eventName;
  set eventName(String? eventName) => _$this._eventName = eventName;

  String? _failureStage;
  String? get failureStage => _$this._failureStage;
  set failureStage(String? failureStage) => _$this._failureStage = failureStage;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _message;
  String? get message => _$this._message;
  set message(String? message) => _$this._message = message;

  String? _module;
  String? get module => _$this._module;
  set module(String? module) => _$this._module = module;

  String? _moduleName;
  String? get moduleName => _$this._moduleName;
  set moduleName(String? moduleName) => _$this._moduleName = moduleName;

  String? _stage;
  String? get stage => _$this._stage;
  set stage(String? stage) => _$this._stage = stage;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _step;
  String? get step => _$this._step;
  set step(String? step) => _$this._step = step;

  String? _stepId;
  String? get stepId => _$this._stepId;
  set stepId(String? stepId) => _$this._stepId = stepId;

  DeploymentEventResponseBuilder() {
    DeploymentEventResponse._defaults(this);
  }

  DeploymentEventResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _eventName = $v.eventName;
      _failureStage = $v.failureStage;
      _id = $v.id;
      _message = $v.message;
      _module = $v.module;
      _moduleName = $v.moduleName;
      _stage = $v.stage;
      _status = $v.status;
      _step = $v.step;
      _stepId = $v.stepId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeploymentEventResponse other) {
    _$v = other as _$DeploymentEventResponse;
  }

  @override
  void update(void Function(DeploymentEventResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeploymentEventResponse build() => _build();

  _$DeploymentEventResponse _build() {
    final _$result =
        _$v ??
        _$DeploymentEventResponse._(
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'DeploymentEventResponse',
            'createdAt',
          ),
          eventName: BuiltValueNullFieldError.checkNotNull(
            eventName,
            r'DeploymentEventResponse',
            'eventName',
          ),
          failureStage: failureStage,
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'DeploymentEventResponse',
            'id',
          ),
          message: message,
          module: module,
          moduleName: moduleName,
          stage: stage,
          status: status,
          step: step,
          stepId: stepId,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
