// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentResponse extends AgentResponse {
  @override
  final String? agentVersion;
  @override
  final String? architecture;
  @override
  final String createdAt;
  @override
  final String environment;
  @override
  final String? hostname;
  @override
  final String id;
  @override
  final String? lastSeenAt;
  @override
  final String name;
  @override
  final String nodeId;
  @override
  final String? registeredAt;
  @override
  final String? revokedAt;
  @override
  final String status;

  factory _$AgentResponse([void Function(AgentResponseBuilder)? updates]) =>
      (AgentResponseBuilder()..update(updates))._build();

  _$AgentResponse._({
    this.agentVersion,
    this.architecture,
    required this.createdAt,
    required this.environment,
    this.hostname,
    required this.id,
    this.lastSeenAt,
    required this.name,
    required this.nodeId,
    this.registeredAt,
    this.revokedAt,
    required this.status,
  }) : super._();
  @override
  AgentResponse rebuild(void Function(AgentResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AgentResponseBuilder toBuilder() => AgentResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentResponse &&
        agentVersion == other.agentVersion &&
        architecture == other.architecture &&
        createdAt == other.createdAt &&
        environment == other.environment &&
        hostname == other.hostname &&
        id == other.id &&
        lastSeenAt == other.lastSeenAt &&
        name == other.name &&
        nodeId == other.nodeId &&
        registeredAt == other.registeredAt &&
        revokedAt == other.revokedAt &&
        status == other.status;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, agentVersion.hashCode);
    _$hash = $jc(_$hash, architecture.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, environment.hashCode);
    _$hash = $jc(_$hash, hostname.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, lastSeenAt.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, registeredAt.hashCode);
    _$hash = $jc(_$hash, revokedAt.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentResponse')
          ..add('agentVersion', agentVersion)
          ..add('architecture', architecture)
          ..add('createdAt', createdAt)
          ..add('environment', environment)
          ..add('hostname', hostname)
          ..add('id', id)
          ..add('lastSeenAt', lastSeenAt)
          ..add('name', name)
          ..add('nodeId', nodeId)
          ..add('registeredAt', registeredAt)
          ..add('revokedAt', revokedAt)
          ..add('status', status))
        .toString();
  }
}

class AgentResponseBuilder
    implements Builder<AgentResponse, AgentResponseBuilder> {
  _$AgentResponse? _$v;

  String? _agentVersion;
  String? get agentVersion => _$this._agentVersion;
  set agentVersion(String? agentVersion) => _$this._agentVersion = agentVersion;

  String? _architecture;
  String? get architecture => _$this._architecture;
  set architecture(String? architecture) => _$this._architecture = architecture;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _environment;
  String? get environment => _$this._environment;
  set environment(String? environment) => _$this._environment = environment;

  String? _hostname;
  String? get hostname => _$this._hostname;
  set hostname(String? hostname) => _$this._hostname = hostname;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _lastSeenAt;
  String? get lastSeenAt => _$this._lastSeenAt;
  set lastSeenAt(String? lastSeenAt) => _$this._lastSeenAt = lastSeenAt;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  String? _registeredAt;
  String? get registeredAt => _$this._registeredAt;
  set registeredAt(String? registeredAt) => _$this._registeredAt = registeredAt;

  String? _revokedAt;
  String? get revokedAt => _$this._revokedAt;
  set revokedAt(String? revokedAt) => _$this._revokedAt = revokedAt;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  AgentResponseBuilder() {
    AgentResponse._defaults(this);
  }

  AgentResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agentVersion = $v.agentVersion;
      _architecture = $v.architecture;
      _createdAt = $v.createdAt;
      _environment = $v.environment;
      _hostname = $v.hostname;
      _id = $v.id;
      _lastSeenAt = $v.lastSeenAt;
      _name = $v.name;
      _nodeId = $v.nodeId;
      _registeredAt = $v.registeredAt;
      _revokedAt = $v.revokedAt;
      _status = $v.status;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentResponse other) {
    _$v = other as _$AgentResponse;
  }

  @override
  void update(void Function(AgentResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentResponse build() => _build();

  _$AgentResponse _build() {
    final _$result =
        _$v ??
        _$AgentResponse._(
          agentVersion: agentVersion,
          architecture: architecture,
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'AgentResponse',
            'createdAt',
          ),
          environment: BuiltValueNullFieldError.checkNotNull(
            environment,
            r'AgentResponse',
            'environment',
          ),
          hostname: hostname,
          id: BuiltValueNullFieldError.checkNotNull(id, r'AgentResponse', 'id'),
          lastSeenAt: lastSeenAt,
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'AgentResponse',
            'name',
          ),
          nodeId: BuiltValueNullFieldError.checkNotNull(
            nodeId,
            r'AgentResponse',
            'nodeId',
          ),
          registeredAt: registeredAt,
          revokedAt: revokedAt,
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'AgentResponse',
            'status',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
