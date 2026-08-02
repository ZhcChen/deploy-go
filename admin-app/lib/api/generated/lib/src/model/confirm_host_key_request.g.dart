// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'confirm_host_key_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ConfirmHostKeyRequest extends ConfirmHostKeyRequest {
  @override
  final String checkId;
  @override
  final String snapshotHash;
  @override
  final int version;

  factory _$ConfirmHostKeyRequest(
          [void Function(ConfirmHostKeyRequestBuilder)? updates]) =>
      (ConfirmHostKeyRequestBuilder()..update(updates))._build();

  _$ConfirmHostKeyRequest._(
      {required this.checkId,
      required this.snapshotHash,
      required this.version})
      : super._();
  @override
  ConfirmHostKeyRequest rebuild(
          void Function(ConfirmHostKeyRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ConfirmHostKeyRequestBuilder toBuilder() =>
      ConfirmHostKeyRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ConfirmHostKeyRequest &&
        checkId == other.checkId &&
        snapshotHash == other.snapshotHash &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, checkId.hashCode);
    _$hash = $jc(_$hash, snapshotHash.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ConfirmHostKeyRequest')
          ..add('checkId', checkId)
          ..add('snapshotHash', snapshotHash)
          ..add('version', version))
        .toString();
  }
}

class ConfirmHostKeyRequestBuilder
    implements Builder<ConfirmHostKeyRequest, ConfirmHostKeyRequestBuilder> {
  _$ConfirmHostKeyRequest? _$v;

  String? _checkId;
  String? get checkId => _$this._checkId;
  set checkId(String? checkId) => _$this._checkId = checkId;

  String? _snapshotHash;
  String? get snapshotHash => _$this._snapshotHash;
  set snapshotHash(String? snapshotHash) => _$this._snapshotHash = snapshotHash;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  ConfirmHostKeyRequestBuilder() {
    ConfirmHostKeyRequest._defaults(this);
  }

  ConfirmHostKeyRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _checkId = $v.checkId;
      _snapshotHash = $v.snapshotHash;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ConfirmHostKeyRequest other) {
    _$v = other as _$ConfirmHostKeyRequest;
  }

  @override
  void update(void Function(ConfirmHostKeyRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ConfirmHostKeyRequest build() => _build();

  _$ConfirmHostKeyRequest _build() {
    final _$result = _$v ??
        _$ConfirmHostKeyRequest._(
          checkId: BuiltValueNullFieldError.checkNotNull(
              checkId, r'ConfirmHostKeyRequest', 'checkId'),
          snapshotHash: BuiltValueNullFieldError.checkNotNull(
              snapshotHash, r'ConfirmHostKeyRequest', 'snapshotHash'),
          version: BuiltValueNullFieldError.checkNotNull(
              version, r'ConfirmHostKeyRequest', 'version'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
