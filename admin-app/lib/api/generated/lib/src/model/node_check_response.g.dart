// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'node_check_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$NodeCheckResponse extends NodeCheckResponse {
  @override
  final String? architecture;
  @override
  final String createdAt;
  @override
  final int? diskAvailableBytes;
  @override
  final String? failureCode;
  @override
  final String? failureMessage;
  @override
  final String? finishedAt;
  @override
  final String id;
  @override
  final String? osName;
  @override
  final String status;

  factory _$NodeCheckResponse([
    void Function(NodeCheckResponseBuilder)? updates,
  ]) => (NodeCheckResponseBuilder()..update(updates))._build();

  _$NodeCheckResponse._({
    this.architecture,
    required this.createdAt,
    this.diskAvailableBytes,
    this.failureCode,
    this.failureMessage,
    this.finishedAt,
    required this.id,
    this.osName,
    required this.status,
  }) : super._();
  @override
  NodeCheckResponse rebuild(void Function(NodeCheckResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  NodeCheckResponseBuilder toBuilder() =>
      NodeCheckResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is NodeCheckResponse &&
        architecture == other.architecture &&
        createdAt == other.createdAt &&
        diskAvailableBytes == other.diskAvailableBytes &&
        failureCode == other.failureCode &&
        failureMessage == other.failureMessage &&
        finishedAt == other.finishedAt &&
        id == other.id &&
        osName == other.osName &&
        status == other.status;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, architecture.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, diskAvailableBytes.hashCode);
    _$hash = $jc(_$hash, failureCode.hashCode);
    _$hash = $jc(_$hash, failureMessage.hashCode);
    _$hash = $jc(_$hash, finishedAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, osName.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'NodeCheckResponse')
          ..add('architecture', architecture)
          ..add('createdAt', createdAt)
          ..add('diskAvailableBytes', diskAvailableBytes)
          ..add('failureCode', failureCode)
          ..add('failureMessage', failureMessage)
          ..add('finishedAt', finishedAt)
          ..add('id', id)
          ..add('osName', osName)
          ..add('status', status))
        .toString();
  }
}

class NodeCheckResponseBuilder
    implements Builder<NodeCheckResponse, NodeCheckResponseBuilder> {
  _$NodeCheckResponse? _$v;

  String? _architecture;
  String? get architecture => _$this._architecture;
  set architecture(String? architecture) => _$this._architecture = architecture;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  int? _diskAvailableBytes;
  int? get diskAvailableBytes => _$this._diskAvailableBytes;
  set diskAvailableBytes(int? diskAvailableBytes) =>
      _$this._diskAvailableBytes = diskAvailableBytes;

  String? _failureCode;
  String? get failureCode => _$this._failureCode;
  set failureCode(String? failureCode) => _$this._failureCode = failureCode;

  String? _failureMessage;
  String? get failureMessage => _$this._failureMessage;
  set failureMessage(String? failureMessage) =>
      _$this._failureMessage = failureMessage;

  String? _finishedAt;
  String? get finishedAt => _$this._finishedAt;
  set finishedAt(String? finishedAt) => _$this._finishedAt = finishedAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _osName;
  String? get osName => _$this._osName;
  set osName(String? osName) => _$this._osName = osName;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  NodeCheckResponseBuilder() {
    NodeCheckResponse._defaults(this);
  }

  NodeCheckResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _architecture = $v.architecture;
      _createdAt = $v.createdAt;
      _diskAvailableBytes = $v.diskAvailableBytes;
      _failureCode = $v.failureCode;
      _failureMessage = $v.failureMessage;
      _finishedAt = $v.finishedAt;
      _id = $v.id;
      _osName = $v.osName;
      _status = $v.status;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(NodeCheckResponse other) {
    _$v = other as _$NodeCheckResponse;
  }

  @override
  void update(void Function(NodeCheckResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  NodeCheckResponse build() => _build();

  _$NodeCheckResponse _build() {
    final _$result =
        _$v ??
        _$NodeCheckResponse._(
          architecture: architecture,
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'NodeCheckResponse',
            'createdAt',
          ),
          diskAvailableBytes: diskAvailableBytes,
          failureCode: failureCode,
          failureMessage: failureMessage,
          finishedAt: finishedAt,
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'NodeCheckResponse',
            'id',
          ),
          osName: osName,
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'NodeCheckResponse',
            'status',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
