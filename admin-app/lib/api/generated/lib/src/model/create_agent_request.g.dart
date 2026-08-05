// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_agent_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateAgentRequest extends CreateAgentRequest {
  @override
  final String environment;
  @override
  final String name;
  @override
  final String? nodeId;

  factory _$CreateAgentRequest([
    void Function(CreateAgentRequestBuilder)? updates,
  ]) => (CreateAgentRequestBuilder()..update(updates))._build();

  _$CreateAgentRequest._({
    required this.environment,
    required this.name,
    this.nodeId,
  }) : super._();
  @override
  CreateAgentRequest rebuild(
    void Function(CreateAgentRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  CreateAgentRequestBuilder toBuilder() =>
      CreateAgentRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateAgentRequest &&
        environment == other.environment &&
        name == other.name &&
        nodeId == other.nodeId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, environment.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreateAgentRequest')
          ..add('environment', environment)
          ..add('name', name)
          ..add('nodeId', nodeId))
        .toString();
  }
}

class CreateAgentRequestBuilder
    implements Builder<CreateAgentRequest, CreateAgentRequestBuilder> {
  _$CreateAgentRequest? _$v;

  String? _environment;
  String? get environment => _$this._environment;
  set environment(String? environment) => _$this._environment = environment;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  CreateAgentRequestBuilder() {
    CreateAgentRequest._defaults(this);
  }

  CreateAgentRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _environment = $v.environment;
      _name = $v.name;
      _nodeId = $v.nodeId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateAgentRequest other) {
    _$v = other as _$CreateAgentRequest;
  }

  @override
  void update(void Function(CreateAgentRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateAgentRequest build() => _build();

  _$CreateAgentRequest _build() {
    final _$result =
        _$v ??
        _$CreateAgentRequest._(
          environment: BuiltValueNullFieldError.checkNotNull(
            environment,
            r'CreateAgentRequest',
            'environment',
          ),
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'CreateAgentRequest',
            'name',
          ),
          nodeId: nodeId,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
