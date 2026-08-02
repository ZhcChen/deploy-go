// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'host_key_scan_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$HostKeyScanResponse extends HostKeyScanResponse {
  @override
  final String checkId;
  @override
  final String fingerprint;
  @override
  final String snapshotHash;

  factory _$HostKeyScanResponse(
          [void Function(HostKeyScanResponseBuilder)? updates]) =>
      (HostKeyScanResponseBuilder()..update(updates))._build();

  _$HostKeyScanResponse._(
      {required this.checkId,
      required this.fingerprint,
      required this.snapshotHash})
      : super._();
  @override
  HostKeyScanResponse rebuild(
          void Function(HostKeyScanResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  HostKeyScanResponseBuilder toBuilder() =>
      HostKeyScanResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is HostKeyScanResponse &&
        checkId == other.checkId &&
        fingerprint == other.fingerprint &&
        snapshotHash == other.snapshotHash;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, checkId.hashCode);
    _$hash = $jc(_$hash, fingerprint.hashCode);
    _$hash = $jc(_$hash, snapshotHash.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'HostKeyScanResponse')
          ..add('checkId', checkId)
          ..add('fingerprint', fingerprint)
          ..add('snapshotHash', snapshotHash))
        .toString();
  }
}

class HostKeyScanResponseBuilder
    implements Builder<HostKeyScanResponse, HostKeyScanResponseBuilder> {
  _$HostKeyScanResponse? _$v;

  String? _checkId;
  String? get checkId => _$this._checkId;
  set checkId(String? checkId) => _$this._checkId = checkId;

  String? _fingerprint;
  String? get fingerprint => _$this._fingerprint;
  set fingerprint(String? fingerprint) => _$this._fingerprint = fingerprint;

  String? _snapshotHash;
  String? get snapshotHash => _$this._snapshotHash;
  set snapshotHash(String? snapshotHash) => _$this._snapshotHash = snapshotHash;

  HostKeyScanResponseBuilder() {
    HostKeyScanResponse._defaults(this);
  }

  HostKeyScanResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _checkId = $v.checkId;
      _fingerprint = $v.fingerprint;
      _snapshotHash = $v.snapshotHash;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(HostKeyScanResponse other) {
    _$v = other as _$HostKeyScanResponse;
  }

  @override
  void update(void Function(HostKeyScanResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  HostKeyScanResponse build() => _build();

  _$HostKeyScanResponse _build() {
    final _$result = _$v ??
        _$HostKeyScanResponse._(
          checkId: BuiltValueNullFieldError.checkNotNull(
              checkId, r'HostKeyScanResponse', 'checkId'),
          fingerprint: BuiltValueNullFieldError.checkNotNull(
              fingerprint, r'HostKeyScanResponse', 'fingerprint'),
          snapshotHash: BuiltValueNullFieldError.checkNotNull(
              snapshotHash, r'HostKeyScanResponse', 'snapshotHash'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
