//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'upload_status_response.g.dart';

/// UploadStatusResponse
///
/// Properties:
/// * [artifactId]
/// * [leaseId]
/// * [offset]
/// * [status]
/// * [uploadSize]
@BuiltValue()
abstract class UploadStatusResponse implements Built<UploadStatusResponse, UploadStatusResponseBuilder> {
  @BuiltValueField(wireName: r'artifact_id')
  String get artifactId;

  @BuiltValueField(wireName: r'lease_id')
  String get leaseId;

  @BuiltValueField(wireName: r'offset')
  int get offset;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'upload_size')
  int get uploadSize;

  UploadStatusResponse._();

  factory UploadStatusResponse([void updates(UploadStatusResponseBuilder b)]) = _$UploadStatusResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UploadStatusResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UploadStatusResponse> get serializer => _$UploadStatusResponseSerializer();
}

class _$UploadStatusResponseSerializer implements PrimitiveSerializer<UploadStatusResponse> {
  @override
  final Iterable<Type> types = const [UploadStatusResponse, _$UploadStatusResponse];

  @override
  final String wireName = r'UploadStatusResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UploadStatusResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'artifact_id';
    yield serializers.serialize(
      object.artifactId,
      specifiedType: const FullType(String),
    );
    yield r'lease_id';
    yield serializers.serialize(
      object.leaseId,
      specifiedType: const FullType(String),
    );
    yield r'offset';
    yield serializers.serialize(
      object.offset,
      specifiedType: const FullType(int),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
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
    UploadStatusResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required UploadStatusResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'artifact_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.artifactId = valueDes;
          break;
        case r'lease_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.leaseId = valueDes;
          break;
        case r'offset':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.offset = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
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
  UploadStatusResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UploadStatusResponseBuilder();
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
