// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'initialize_application_configs_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$InitializeApplicationConfigsResponse
    extends InitializeApplicationConfigsResponse {
  @override
  final String bindingId;
  @override
  final bool created;
  @override
  final int fileCount;
  @override
  final String status;

  factory _$InitializeApplicationConfigsResponse([
    void Function(InitializeApplicationConfigsResponseBuilder)? updates,
  ]) =>
      (InitializeApplicationConfigsResponseBuilder()..update(updates))._build();

  _$InitializeApplicationConfigsResponse._({
    required this.bindingId,
    required this.created,
    required this.fileCount,
    required this.status,
  }) : super._();
  @override
  InitializeApplicationConfigsResponse rebuild(
    void Function(InitializeApplicationConfigsResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  InitializeApplicationConfigsResponseBuilder toBuilder() =>
      InitializeApplicationConfigsResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is InitializeApplicationConfigsResponse &&
        bindingId == other.bindingId &&
        created == other.created &&
        fileCount == other.fileCount &&
        status == other.status;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, bindingId.hashCode);
    _$hash = $jc(_$hash, created.hashCode);
    _$hash = $jc(_$hash, fileCount.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'InitializeApplicationConfigsResponse')
          ..add('bindingId', bindingId)
          ..add('created', created)
          ..add('fileCount', fileCount)
          ..add('status', status))
        .toString();
  }
}

class InitializeApplicationConfigsResponseBuilder
    implements
        Builder<
          InitializeApplicationConfigsResponse,
          InitializeApplicationConfigsResponseBuilder
        > {
  _$InitializeApplicationConfigsResponse? _$v;

  String? _bindingId;
  String? get bindingId => _$this._bindingId;
  set bindingId(String? bindingId) => _$this._bindingId = bindingId;

  bool? _created;
  bool? get created => _$this._created;
  set created(bool? created) => _$this._created = created;

  int? _fileCount;
  int? get fileCount => _$this._fileCount;
  set fileCount(int? fileCount) => _$this._fileCount = fileCount;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  InitializeApplicationConfigsResponseBuilder() {
    InitializeApplicationConfigsResponse._defaults(this);
  }

  InitializeApplicationConfigsResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _bindingId = $v.bindingId;
      _created = $v.created;
      _fileCount = $v.fileCount;
      _status = $v.status;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(InitializeApplicationConfigsResponse other) {
    _$v = other as _$InitializeApplicationConfigsResponse;
  }

  @override
  void update(
    void Function(InitializeApplicationConfigsResponseBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  InitializeApplicationConfigsResponse build() => _build();

  _$InitializeApplicationConfigsResponse _build() {
    final _$result =
        _$v ??
        _$InitializeApplicationConfigsResponse._(
          bindingId: BuiltValueNullFieldError.checkNotNull(
            bindingId,
            r'InitializeApplicationConfigsResponse',
            'bindingId',
          ),
          created: BuiltValueNullFieldError.checkNotNull(
            created,
            r'InitializeApplicationConfigsResponse',
            'created',
          ),
          fileCount: BuiltValueNullFieldError.checkNotNull(
            fileCount,
            r'InitializeApplicationConfigsResponse',
            'fileCount',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'InitializeApplicationConfigsResponse',
            'status',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
