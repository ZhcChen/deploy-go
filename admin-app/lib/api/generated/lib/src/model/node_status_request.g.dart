// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'node_status_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$NodeStatusRequest extends NodeStatusRequest {
  @override
  final String status;
  @override
  final int version;

  factory _$NodeStatusRequest(
          [void Function(NodeStatusRequestBuilder)? updates]) =>
      (NodeStatusRequestBuilder()..update(updates))._build();

  _$NodeStatusRequest._({required this.status, required this.version})
      : super._();
  @override
  NodeStatusRequest rebuild(void Function(NodeStatusRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  NodeStatusRequestBuilder toBuilder() =>
      NodeStatusRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is NodeStatusRequest &&
        status == other.status &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'NodeStatusRequest')
          ..add('status', status)
          ..add('version', version))
        .toString();
  }
}

class NodeStatusRequestBuilder
    implements Builder<NodeStatusRequest, NodeStatusRequestBuilder> {
  _$NodeStatusRequest? _$v;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  NodeStatusRequestBuilder() {
    NodeStatusRequest._defaults(this);
  }

  NodeStatusRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _status = $v.status;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(NodeStatusRequest other) {
    _$v = other as _$NodeStatusRequest;
  }

  @override
  void update(void Function(NodeStatusRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  NodeStatusRequest build() => _build();

  _$NodeStatusRequest _build() {
    final _$result = _$v ??
        _$NodeStatusRequest._(
          status: BuiltValueNullFieldError.checkNotNull(
              status, r'NodeStatusRequest', 'status'),
          version: BuiltValueNullFieldError.checkNotNull(
              version, r'NodeStatusRequest', 'version'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
