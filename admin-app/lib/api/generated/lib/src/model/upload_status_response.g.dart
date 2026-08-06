// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'upload_status_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UploadStatusResponse extends UploadStatusResponse {
  @override
  final String artifactId;
  @override
  final String leaseId;
  @override
  final int offset;
  @override
  final String status;
  @override
  final int uploadSize;

  factory _$UploadStatusResponse([
    void Function(UploadStatusResponseBuilder)? updates,
  ]) => (UploadStatusResponseBuilder()..update(updates))._build();

  _$UploadStatusResponse._({
    required this.artifactId,
    required this.leaseId,
    required this.offset,
    required this.status,
    required this.uploadSize,
  }) : super._();
  @override
  UploadStatusResponse rebuild(
    void Function(UploadStatusResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  UploadStatusResponseBuilder toBuilder() =>
      UploadStatusResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UploadStatusResponse &&
        artifactId == other.artifactId &&
        leaseId == other.leaseId &&
        offset == other.offset &&
        status == other.status &&
        uploadSize == other.uploadSize;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, artifactId.hashCode);
    _$hash = $jc(_$hash, leaseId.hashCode);
    _$hash = $jc(_$hash, offset.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, uploadSize.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UploadStatusResponse')
          ..add('artifactId', artifactId)
          ..add('leaseId', leaseId)
          ..add('offset', offset)
          ..add('status', status)
          ..add('uploadSize', uploadSize))
        .toString();
  }
}

class UploadStatusResponseBuilder
    implements Builder<UploadStatusResponse, UploadStatusResponseBuilder> {
  _$UploadStatusResponse? _$v;

  String? _artifactId;
  String? get artifactId => _$this._artifactId;
  set artifactId(String? artifactId) => _$this._artifactId = artifactId;

  String? _leaseId;
  String? get leaseId => _$this._leaseId;
  set leaseId(String? leaseId) => _$this._leaseId = leaseId;

  int? _offset;
  int? get offset => _$this._offset;
  set offset(int? offset) => _$this._offset = offset;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  int? _uploadSize;
  int? get uploadSize => _$this._uploadSize;
  set uploadSize(int? uploadSize) => _$this._uploadSize = uploadSize;

  UploadStatusResponseBuilder() {
    UploadStatusResponse._defaults(this);
  }

  UploadStatusResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _artifactId = $v.artifactId;
      _leaseId = $v.leaseId;
      _offset = $v.offset;
      _status = $v.status;
      _uploadSize = $v.uploadSize;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UploadStatusResponse other) {
    _$v = other as _$UploadStatusResponse;
  }

  @override
  void update(void Function(UploadStatusResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UploadStatusResponse build() => _build();

  _$UploadStatusResponse _build() {
    final _$result =
        _$v ??
        _$UploadStatusResponse._(
          artifactId: BuiltValueNullFieldError.checkNotNull(
            artifactId,
            r'UploadStatusResponse',
            'artifactId',
          ),
          leaseId: BuiltValueNullFieldError.checkNotNull(
            leaseId,
            r'UploadStatusResponse',
            'leaseId',
          ),
          offset: BuiltValueNullFieldError.checkNotNull(
            offset,
            r'UploadStatusResponse',
            'offset',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'UploadStatusResponse',
            'status',
          ),
          uploadSize: BuiltValueNullFieldError.checkNotNull(
            uploadSize,
            r'UploadStatusResponse',
            'uploadSize',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
