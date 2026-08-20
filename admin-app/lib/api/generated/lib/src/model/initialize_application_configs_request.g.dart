// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'initialize_application_configs_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$InitializeApplicationConfigsRequest
    extends InitializeApplicationConfigsRequest {
  @override
  final String targetId;
  @override
  final String? templateId;

  factory _$InitializeApplicationConfigsRequest([
    void Function(InitializeApplicationConfigsRequestBuilder)? updates,
  ]) =>
      (InitializeApplicationConfigsRequestBuilder()..update(updates))._build();

  _$InitializeApplicationConfigsRequest._({
    required this.targetId,
    this.templateId,
  }) : super._();
  @override
  InitializeApplicationConfigsRequest rebuild(
    void Function(InitializeApplicationConfigsRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  InitializeApplicationConfigsRequestBuilder toBuilder() =>
      InitializeApplicationConfigsRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is InitializeApplicationConfigsRequest &&
        targetId == other.targetId &&
        templateId == other.templateId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, targetId.hashCode);
    _$hash = $jc(_$hash, templateId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'InitializeApplicationConfigsRequest')
          ..add('targetId', targetId)
          ..add('templateId', templateId))
        .toString();
  }
}

class InitializeApplicationConfigsRequestBuilder
    implements
        Builder<
          InitializeApplicationConfigsRequest,
          InitializeApplicationConfigsRequestBuilder
        > {
  _$InitializeApplicationConfigsRequest? _$v;

  String? _targetId;
  String? get targetId => _$this._targetId;
  set targetId(String? targetId) => _$this._targetId = targetId;

  String? _templateId;
  String? get templateId => _$this._templateId;
  set templateId(String? templateId) => _$this._templateId = templateId;

  InitializeApplicationConfigsRequestBuilder() {
    InitializeApplicationConfigsRequest._defaults(this);
  }

  InitializeApplicationConfigsRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _targetId = $v.targetId;
      _templateId = $v.templateId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(InitializeApplicationConfigsRequest other) {
    _$v = other as _$InitializeApplicationConfigsRequest;
  }

  @override
  void update(
    void Function(InitializeApplicationConfigsRequestBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  InitializeApplicationConfigsRequest build() => _build();

  _$InitializeApplicationConfigsRequest _build() {
    final _$result =
        _$v ??
        _$InitializeApplicationConfigsRequest._(
          targetId: BuiltValueNullFieldError.checkNotNull(
            targetId,
            r'InitializeApplicationConfigsRequest',
            'targetId',
          ),
          templateId: templateId,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
