// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'initiate_upload_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$InitiateUploadRequest extends InitiateUploadRequest {
  @override
  final String archiveDigest;
  @override
  final int uploadSize;

  factory _$InitiateUploadRequest([
    void Function(InitiateUploadRequestBuilder)? updates,
  ]) => (InitiateUploadRequestBuilder()..update(updates))._build();

  _$InitiateUploadRequest._({
    required this.archiveDigest,
    required this.uploadSize,
  }) : super._();
  @override
  InitiateUploadRequest rebuild(
    void Function(InitiateUploadRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  InitiateUploadRequestBuilder toBuilder() =>
      InitiateUploadRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is InitiateUploadRequest &&
        archiveDigest == other.archiveDigest &&
        uploadSize == other.uploadSize;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, archiveDigest.hashCode);
    _$hash = $jc(_$hash, uploadSize.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'InitiateUploadRequest')
          ..add('archiveDigest', archiveDigest)
          ..add('uploadSize', uploadSize))
        .toString();
  }
}

class InitiateUploadRequestBuilder
    implements Builder<InitiateUploadRequest, InitiateUploadRequestBuilder> {
  _$InitiateUploadRequest? _$v;

  String? _archiveDigest;
  String? get archiveDigest => _$this._archiveDigest;
  set archiveDigest(String? archiveDigest) =>
      _$this._archiveDigest = archiveDigest;

  int? _uploadSize;
  int? get uploadSize => _$this._uploadSize;
  set uploadSize(int? uploadSize) => _$this._uploadSize = uploadSize;

  InitiateUploadRequestBuilder() {
    InitiateUploadRequest._defaults(this);
  }

  InitiateUploadRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _archiveDigest = $v.archiveDigest;
      _uploadSize = $v.uploadSize;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(InitiateUploadRequest other) {
    _$v = other as _$InitiateUploadRequest;
  }

  @override
  void update(void Function(InitiateUploadRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  InitiateUploadRequest build() => _build();

  _$InitiateUploadRequest _build() {
    final _$result =
        _$v ??
        _$InitiateUploadRequest._(
          archiveDigest: BuiltValueNullFieldError.checkNotNull(
            archiveDigest,
            r'InitiateUploadRequest',
            'archiveDigest',
          ),
          uploadSize: BuiltValueNullFieldError.checkNotNull(
            uploadSize,
            r'InitiateUploadRequest',
            'uploadSize',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
