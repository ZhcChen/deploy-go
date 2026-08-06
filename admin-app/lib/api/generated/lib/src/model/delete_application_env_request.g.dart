// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'delete_application_env_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeleteApplicationEnvRequest extends DeleteApplicationEnvRequest {
  @override
  final String confirmFileName;
  @override
  final int expectedVersion;

  factory _$DeleteApplicationEnvRequest([
    void Function(DeleteApplicationEnvRequestBuilder)? updates,
  ]) => (DeleteApplicationEnvRequestBuilder()..update(updates))._build();

  _$DeleteApplicationEnvRequest._({
    required this.confirmFileName,
    required this.expectedVersion,
  }) : super._();
  @override
  DeleteApplicationEnvRequest rebuild(
    void Function(DeleteApplicationEnvRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  DeleteApplicationEnvRequestBuilder toBuilder() =>
      DeleteApplicationEnvRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeleteApplicationEnvRequest &&
        confirmFileName == other.confirmFileName &&
        expectedVersion == other.expectedVersion;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, confirmFileName.hashCode);
    _$hash = $jc(_$hash, expectedVersion.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'DeleteApplicationEnvRequest')
          ..add('confirmFileName', confirmFileName)
          ..add('expectedVersion', expectedVersion))
        .toString();
  }
}

class DeleteApplicationEnvRequestBuilder
    implements
        Builder<
          DeleteApplicationEnvRequest,
          DeleteApplicationEnvRequestBuilder
        > {
  _$DeleteApplicationEnvRequest? _$v;

  String? _confirmFileName;
  String? get confirmFileName => _$this._confirmFileName;
  set confirmFileName(String? confirmFileName) =>
      _$this._confirmFileName = confirmFileName;

  int? _expectedVersion;
  int? get expectedVersion => _$this._expectedVersion;
  set expectedVersion(int? expectedVersion) =>
      _$this._expectedVersion = expectedVersion;

  DeleteApplicationEnvRequestBuilder() {
    DeleteApplicationEnvRequest._defaults(this);
  }

  DeleteApplicationEnvRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _confirmFileName = $v.confirmFileName;
      _expectedVersion = $v.expectedVersion;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeleteApplicationEnvRequest other) {
    _$v = other as _$DeleteApplicationEnvRequest;
  }

  @override
  void update(void Function(DeleteApplicationEnvRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeleteApplicationEnvRequest build() => _build();

  _$DeleteApplicationEnvRequest _build() {
    final _$result =
        _$v ??
        _$DeleteApplicationEnvRequest._(
          confirmFileName: BuiltValueNullFieldError.checkNotNull(
            confirmFileName,
            r'DeleteApplicationEnvRequest',
            'confirmFileName',
          ),
          expectedVersion: BuiltValueNullFieldError.checkNotNull(
            expectedVersion,
            r'DeleteApplicationEnvRequest',
            'expectedVersion',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
