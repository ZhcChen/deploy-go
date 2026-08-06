//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'initiate_upload_request.g.dart';

/// InitiateUploadRequest
///
/// Properties:
/// * [archiveDigest]
/// * [uploadSize]
@BuiltValue()
abstract class InitiateUploadRequest implements Built<InitiateUploadRequest, InitiateUploadRequestBuilder> {
  @BuiltValueField(wireName: r'archive_digest')
  String get archiveDigest;

  @BuiltValueField(wireName: r'upload_size')
  int get uploadSize;

  InitiateUploadRequest._();

  factory InitiateUploadRequest([void updates(InitiateUploadRequestBuilder b)]) = _$InitiateUploadRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(InitiateUploadRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<InitiateUploadRequest> get serializer => _$InitiateUploadRequestSerializer();
}

class _$InitiateUploadRequestSerializer implements PrimitiveSerializer<InitiateUploadRequest> {
  @override
  final Iterable<Type> types = const [InitiateUploadRequest, _$InitiateUploadRequest];

  @override
  final String wireName = r'InitiateUploadRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    InitiateUploadRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'archive_digest';
    yield serializers.serialize(
      object.archiveDigest,
      specifiedType: const FullType(String),
    );
    yield r'upload_size';
    yield serializers.serialize(
      object.uploadSize,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    InitiateUploadRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required InitiateUploadRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'archive_digest':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.archiveDigest = valueDes;
          break;
        case r'upload_size':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.uploadSize = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  InitiateUploadRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = InitiateUploadRequestBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
